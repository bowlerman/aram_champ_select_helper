use futures::stream::StreamExt;
use mongodb::bson::{doc, Bson};
use mongodb::options::{UpdateOptions, ClientOptions};
use mongodb::{Collection, Database, IndexModel, Client};
use riven::consts::{Champion, Queue, RegionalRoute};
use riven::models::match_v5::Match;
use riven::RiotApi;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::convert::Infallible;
use std::error::Error;
use std::fmt::Display;
use std::hash::Hash;
use tokio::try_join;
use summoners::{insert_first_summoner, get_summoner_id, SummonerDocument, insert_summoner_by_puuid};
use utils::get_current_unix_time;

mod summoners;
mod utils;

#[derive(Debug, Clone)]
enum PlayerTeam {
    Blue,
    Red,
}

impl Display for PlayerTeam {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let team_str = match self {
            PlayerTeam::Blue => "blue",
            PlayerTeam::Red => "red",
        };
        write!(f, "{team_str}")
    }
}



#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ARAMMatchDocument {
    pub blue_champs: [Champion; 5], // champs of Team 100
    pub red_champs: [Champion; 5],  // champs of Team 200
    pub blue_win: bool,
    pub match_id: String,
    pub game_start: i64,
}

impl From<ARAMMatchDocument> for Bson {
    fn from(m_doc: ARAMMatchDocument) -> Self {
        Bson::Document(
            doc! {"blue_champs": Into::<Vec<Bson>>::into(m_doc.blue_champs.map(|c| Into::<Bson>::into(Into::<i16>::into(c) as i32))) , "red_champs": Into::<Vec<Bson>>::into(m_doc.red_champs.map(|c| Into::<Bson>::into(Into::<i16>::into(c) as i32))), "blue_win": m_doc.blue_win, "match_id": m_doc.match_id, "game_start": m_doc.game_start},
        )
    }
}

#[derive(Debug, Clone)]
struct MonsterParticipant {
    match_id: String,
    summoner_name: String,
}

impl Display for MonsterParticipant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "participant {} in match {} is not on blue or red team",
            self.summoner_name, self.match_id
        )
    }
}

impl Error for MonsterParticipant {}

#[derive(Debug, Clone)]
struct NoWinner {
    match_id: String,
}

impl Display for NoWinner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "match {} had no winning team", self.match_id)
    }
}

impl Error for NoWinner {}

#[derive(Debug, Clone)]
struct Not5Players {
    match_id: String,
    players: usize,
    team: PlayerTeam,
}

impl Display for Not5Players {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "match {} had {} players on {} team, should be 5",
            self.match_id, self.players, self.team
        )
    }
}

impl Error for Not5Players {}

#[derive(Debug, Clone)]
struct Not10Players {
    match_id: i64,
    players: usize,
}

impl Display for Not10Players {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "match {} had {} participants total, should be 10",
            self.match_id, self.players
        )
    }
}

impl Error for Not10Players {}

pub fn get_match_data_from_match(matc: &Match) -> Result<ARAMMatchDocument, Box<dyn Error>> {
    let match_id = matc.metadata.match_id.clone();
    let mut blue_champs = vec![];
    let mut red_champs = vec![];
    for participant in &matc.info.participants {
        match participant.team_id {
            riven::consts::Team::BLUE => blue_champs.push(participant.champion()?),
            riven::consts::Team::RED => red_champs.push(participant.champion()?),
            riven::consts::Team::OTHER => {
                return Err(Box::new(MonsterParticipant {
                    match_id,
                    summoner_name: participant.summoner_name.clone(),
                }))
            }
        }
    }
    let maybe_5blue_champs: Result<[Champion; 5], _> = blue_champs.try_into();
    let blue_champs = match maybe_5blue_champs {
        Ok(x) => x,
        Err(x) => {
            return Err(Box::new(Not5Players {
                match_id,
                players: x.len(),
                team: PlayerTeam::Blue,
            }))
        }
    };
    let maybe_5red_champs: Result<[Champion; 5], _> = red_champs.try_into();
    let red_champs = match maybe_5red_champs {
        Ok(x) => x,
        Err(x) => {
            return Err(Box::new(Not5Players {
                match_id,
                players: x.len(),
                team: PlayerTeam::Red,
            }))
        }
    };
    let mut blue_win = false;
    let mut red_win = false;
    for team in &matc.info.teams {
        match team.team_id {
            riven::consts::Team::BLUE => blue_win = team.win,
            riven::consts::Team::RED => red_win = team.win,
            riven::consts::Team::OTHER => (),
        }
    }
    let game_start = matc.info.game_start_timestamp;
    if !(blue_win || red_win) {
        return Err(Box::new(NoWinner { match_id }));
    }
    Ok(ARAMMatchDocument {
        blue_champs,
        red_champs,
        blue_win,
        match_id,
        game_start,
    })
}

pub fn get_puuids_from_match(matc: &Match) -> Result<[String; 10], Box<dyn Error>> {
    let mut puuids: Vec<String> = vec![];
    for participant in &matc.info.participants {
        puuids.push(participant.puuid.clone());
    }
    let maybe_10puuids: Result<[String; 10], _> = puuids.try_into();
    let puuids = maybe_10puuids.map_err(|puuids| {
        Box::new(Not10Players {
            match_id: matc.info.game_id,
            players: puuids.len(),
        })
    })?;
    Ok(puuids)
}

pub async fn init_collection_indices(db: &Database) -> Result<(), Box<dyn Error>> {
    let summoners = db.collection::<SummonerDocument>("summoners");
    let matches = db.collection::<ARAMMatchDocument>("matches");
    let summoners_future = summoners.create_indexes(
        vec![
            IndexModel::builder().keys(doc! {"puuid":1}).build(),
            IndexModel::builder()
                .keys(doc! {"time_at_last_fetch":1})
                .build(),
        ],
        None,
    );
    let matches_future = matches.create_indexes(
        vec![
            IndexModel::builder().keys(doc! {"matchid":1}).build(),
            IndexModel::builder().keys(doc! {"game_start":1}).build(),
        ],
        None,
    );
    try_join!(summoners_future, matches_future)?;
    Ok(())
}

pub async fn insert_match_data(
    match_data: ARAMMatchDocument,
    match_collection: &Collection<ARAMMatchDocument>,
) -> Result<(), Box<dyn Error>> {
    match_collection
        .update_one(
            doc! {"match_id": match_data.match_id.clone()},
            doc! { "$setOnInsert": match_data},
            Some(UpdateOptions::builder().upsert(true).build()),
        )
        .await?;
    Ok(())
}

pub async fn get_match_ids(
    puuid: String,
    time_bound: i64,
    riot_api: &mut RiotApi,
) -> Result<Vec<String>, Box<dyn Error>> {
    Ok(riot_api
        .match_v5()
        .get_match_ids_by_puuid(
            RegionalRoute::EUROPE,
            &puuid,
            Some(100),
            Some(get_current_unix_time()),
            Some(Queue::HOWLING_ABYSS_5V5_ARAM),
            Some(get_current_unix_time() - time_bound),
            None,
            None,
        )
        .await?)
}

pub async fn filter_match_ids(
    match_collection: &Collection<ARAMMatchDocument>,
    match_ids: Vec<String>,
) -> Result<Vec<String>, Box<dyn Error>> {
    let old_matches = match_collection
        .find(doc! { "match_id" : { "$in" : match_ids.clone() } }, None)
        .await?
        .filter_map(|e| async move { e.map(|e| e.match_id).ok() })
        .collect::<HashSet<_>>()
        .await;
    let orig_matches = match_ids.into_iter().collect::<HashSet<_>>();
    let new_matches = orig_matches
        .difference(&old_matches)
        .map(|s| s.to_owned())
        .collect::<Vec<String>>();
    Ok(new_matches)
}

#[derive(Debug, Clone)]
struct NonExistentMatchInDB {
    match_id: String,
}

impl Display for NonExistentMatchInDB {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "match {} is in DB but does not exist in Riot DB",
            self.match_id
        )
    }
}

impl Error for NonExistentMatchInDB {}

pub async fn collect_data(seed_summoner: &str, time_limit: i64) -> Result<Infallible, Box<dyn Error>> {
    let client_options = ClientOptions::parse("mongodb://db:27017");
    let client = Client::with_options(client_options.await?)?;
    let db = &client.database("aram_champ_select_helper");
    let summoner_collection = &db.collection("summoners");
    let match_collection = &db.collection("matches");
    init_collection_indices(db).await?;
    let mut riot_api = RiotApi::new(std::env!("RGAPI_KEY"));
    insert_first_summoner(seed_summoner, summoner_collection, &riot_api).await?;
    loop {
        let summoner_puuid = get_summoner_id(summoner_collection, time_limit).await?;
        let match_ids = get_match_ids(summoner_puuid, time_limit, &mut riot_api).await?;
        for match_id in filter_match_ids(match_collection, match_ids).await? {
            let matc = &riot_api
                .match_v5()
                .get_match(RegionalRoute::EUROPE, &match_id)
                .await?
                .ok_or_else(|| Box::new(NonExistentMatchInDB { match_id }))?;
            let match_data = get_match_data_from_match(matc)?;
            let puuids = get_puuids_from_match(matc)?;
            for puuid in puuids {
                insert_summoner_by_puuid(puuid, summoner_collection).await?
            }
            insert_match_data(match_data, match_collection).await?
        }
    }
}