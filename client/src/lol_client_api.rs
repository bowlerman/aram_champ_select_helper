use anyhow::{anyhow, Error};
use async_trait::async_trait;
use league_client_connector::LeagueClientConnector;
use reqwest::{Client, RequestBuilder};
use serde_json::{from_value, Value};
use std::time::Duration;
use tokio::time::interval;

use super::{ARAMChampSelectState, Champ};

#[async_trait]
trait RequestBuilderExt: Sized {
    fn make_lol_client_api_request(addr: &str) -> Result<Self, anyhow::Error>;

    async fn get_json(self) -> Result<Value, anyhow::Error>;

    fn make_lobby_request() -> Result<Self, anyhow::Error> {
        Self::make_lol_client_api_request("lol-champ-select/v1/session")
    }

    async fn try_get_summoner_id() -> Result<u64, anyhow::Error> {
        let result: Value = Self::make_lol_client_api_request("lol-summoner/v1/current-summoner")?
            .get_json()
            .await?;
        let summoner_id = result
            .as_object()
            .ok_or_else(|| anyhow!("Expecting object with summoner info"))?
            .get("summonerId")
            .ok_or_else(|| anyhow!("Expecting summonerId field"))?
            .as_u64()
            .ok_or_else(|| anyhow!("Expecting summoner Id"))?;
        Ok(summoner_id)
    }
}

#[async_trait]
impl RequestBuilderExt for RequestBuilder {
    fn make_lol_client_api_request(addr: &str) -> Result<Self, anyhow::Error> {
        let lock_file = LeagueClientConnector::parse_lockfile()?;
        let request = Client::builder().danger_accept_invalid_certs(true).build()?
        .get(
            format! {"{protocol}://{ip}:{port}/{addr}", ip = lock_file.address, port = lock_file.port, protocol = lock_file.protocol, addr = addr},
        ).header("authorization", format!{"Basic {auth}", auth = lock_file.b64_auth});
        Ok(request)
    }

    async fn get_json(self) -> Result<Value, anyhow::Error> {
        Ok(self.send().await?.json().await?)
    }
}

#[derive(Debug)]
pub struct ChampSelectFetcher {
    request: RequestBuilder,
    summoner_id: u64,
}

impl Clone for ChampSelectFetcher {
    fn clone(&self) -> Self {
        Self {
            request: self.request.try_clone().unwrap(),
            summoner_id: self.summoner_id,
        }
    }
}

impl ChampSelectFetcher {
    pub async fn get_champ_select_state(&self) -> Result<ARAMChampSelectState, Error> {
        let response = self
            .request
            .try_clone()
            .ok_or_else(|| anyhow!("Could not clone champ select api request"))?
            .send()
            .await?;
        let base_json: Value = response.json().await?;
        let json = base_json
            .as_object()
            .ok_or_else(|| anyhow!("Expecting object at top level"))?;
        if let Some(Value::Number(_)) = json.get("httpStatus") {
            Err(anyhow!("Not in champ select"))?
        }
        let bench: Vec<u16> = from_value(json["benchChampionIds"].clone())?;
        let mut team_champs = Vec::new();
        let mut your_champ = 0;
        for (count, member_val) in json["myTeam"]
            .as_array()
            .ok_or_else(|| anyhow!("Expecting list of team members"))?.iter().enumerate()
        {
            let member = member_val
                .as_object()
                .ok_or_else(|| anyhow!("Expecting team member object"))?;
            let champ_id: u16 = from_value(member["championId"].clone())?;
            if member["summonerId"]
                .as_u64()
                .ok_or_else(|| anyhow!("Expecting summoner Id of team member"))?
                == self.summoner_id
            {
                your_champ = count;
            }
            team_champs.push(champ_id);
        }
        let team_champs: [u16; 5] = team_champs.as_slice().try_into()?;
        Ok(ARAMChampSelectState {
            your_champ,
            bench: bench.iter().map(|&id| Champ { id }).collect(),
            team_champs: team_champs.map(|id| Champ { id }),
        })
    }

    pub async fn new() -> Self {
        let mut wait = interval(Duration::from_millis(1000));
        let summoner_id = loop {
            if let (Ok(summoner_id), _) =
                tokio::join!(RequestBuilder::try_get_summoner_id(), wait.tick())
            {
                break summoner_id;
            }
        };
        let request = loop {
            if let Ok(req) = RequestBuilder::make_lobby_request() {
                break req;
            }
            wait.tick().await;
        };
        Self {
            request,
            summoner_id,
        }
    }
}
