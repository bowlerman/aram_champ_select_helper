use std::{error::Error, time::{SystemTime, UNIX_EPOCH}};

use mongodb::{Collection, bson::doc, options::UpdateOptions};
use riven::{RiotApi, consts::PlatformRoute};
use serde::{Serialize, Deserialize};

use crate::utils::get_current_unix_time;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SummonerDocument {
    puuid: String,
    time_at_last_fetch: i64,
}

pub async fn insert_summoner_by_puuid(
    puuid: String,
    summoner_collection: &Collection<SummonerDocument>,
) -> Result<(), Box<dyn Error>> {
    summoner_collection
        .update_one(
            doc! {"puuid": puuid},
            doc! { "$setOnInsert": { "time_at_last_fetch" : 0_i64}},
            Some(UpdateOptions::builder().upsert(true).build()),
        )
        .await?;
    Ok(())
}

pub async fn insert_first_summoner(
    summoner_name: &str,
    summoner_collection: &Collection<SummonerDocument>,
    riot_api: &RiotApi,
) -> Result<(), Box<dyn Error>> {
    let summoner = riot_api
        .summoner_v4()
        .get_by_summoner_name(PlatformRoute::EUW1, summoner_name)
        .await?
        .ok_or_else(|| format!("summoner {} does not exist on the platform", summoner_name))?;
    insert_summoner_by_puuid(summoner.puuid, summoner_collection).await?;
    Ok(())
}

pub async fn get_summoner_id(
    summoner_collection: &Collection<SummonerDocument>,
    time_bound: i64,
) -> Result<String, Box<dyn Error>> {
    let current_time = get_current_unix_time();
    let earliest_time = current_time - time_bound;
    let filter = doc! {"time_at_last_fetch" : {"$lt" : earliest_time} };
    let update = doc! { "$set" : {"time_at_last_fetch" : current_time}};
    Ok(summoner_collection
        .find_one_and_update(filter, update, None)
        .await?
        .ok_or("no_summoner")?
        .puuid)
}