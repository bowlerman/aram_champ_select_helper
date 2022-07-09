use std::{convert::Infallible, error::Error};
use data_collection::collect_data;

const SEED_SUMMONER: &str = "Rock Solid";
const TIME_LIMIT: i64 = 60 * 60 * 24 * 7; // Get maches 1 week back in time

#[tokio::main]
async fn main() -> Result<Infallible, Box<dyn Error>> {
    collect_data(SEED_SUMMONER, TIME_LIMIT).await
}