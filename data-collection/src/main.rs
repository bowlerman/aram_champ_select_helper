use data_collection::*;
use mongodb::{options::ClientOptions, Client};
use riven::{consts::RegionalRoute, RiotApi};
use std::{convert::Infallible, error::Error, fmt::Display};

const SEED_SUMMONER: &str = "Rock Solid";
const TIME_LIMIT: i64 = 60 * 60 * 24 * 7;

#[derive(Debug, Clone)]
struct NonExistentMatchInDB {
    match_id: String,
}

impl Display for NonExistentMatchInDB {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "mafghtch {} is in DB but does not exist in Riot DB",
            self.match_id
        )
    }
}

impl Error for NonExistentMatchInDB {}

#[tokio::main]
async fn main() -> Result<Infallible, Box<dyn Error>> {
    let client_options = ClientOptions::parse("mongodb://db:27017");
    let client = Client::with_options(client_options.await?)?;
    let db = &client.database("aram_champ_select_helper");
    let summoner_collection = &db.collection("summoners");
    let match_collection = &db.collection("matches");
    init_collection_indices(db).await?;
    let mut riot_api = RiotApi::new(std::env!("RGAPI_KEY"));
    insert_first_summoner(SEED_SUMMONER, summoner_collection, &mut riot_api).await?;
    loop {
        let summoner_puuid = get_summoner_id(summoner_collection, TIME_LIMIT).await?;
        let match_ids = get_match_ids(summoner_puuid, TIME_LIMIT, &mut riot_api).await?;
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
