use std::{any::Any, convert::Infallible, error::Error, fs::File, collections::HashMap, hash::Hash, fmt::{Debug, Display}};

use dioxus::prelude::*;
use league_client_connector::LeagueClientConnector;
use reqwest::{Client, RequestBuilder, Response};
use tract_onnx::prelude::*;
use serde_json::{self, Value, from_value};

#[derive(Debug)]
struct LobbyState {
    your_champ: u16,
    bench: [u16; 4],
    team_champs: Vec<u16>
}

type Model = SimplePlan<TypedFact, Box<dyn TypedOp>, Graph<TypedFact, Box<dyn TypedOp>>>;

fn get_win_rate(team: &[u16; 5], champ_dict: &HashMap<u16, usize>, model: &Model) -> Result<f32, Box<dyn Error>> {
    let tot_champs = champ_dict.len();
    let mut one_hot = vec![0_f32; tot_champs + 1];
    for champ in team {
        one_hot[champ_dict[champ]] = 1.;
    }
    let input = tract_ndarray::arr1(&one_hot).into_shape((1, tot_champs + 1))?;
    let tensor_res = model.run(tvec![input.into()])?.to_vec().pop().expect("Expecting model output");
    let res: [f32; 2] = tensor_res.as_slice()?.try_into()?;
    let sum = res[0] + res[1];
    Ok(res[0]/sum)
}

fn map_champ_id_to_index(all_champs: &Vec<(String, u16)>) -> Result<HashMap<u16, usize>, Box<dyn Error>> {
    let mut map = HashMap::new();
    for i in 0..all_champs.len() {
        map.insert(all_champs[i].1, i);
    }
    Ok(map)
}

fn make_lobby_request() -> Result<RequestBuilder, Box<dyn Error>> {
    let lock_file = LeagueClientConnector::parse_lockfile()?;
    let request = Client::builder().danger_accept_invalid_certs(true).build()?
    .get(
        format! {"{protocol}://{ip}:{port}/lol-champ-select/v1/session", ip = lock_file.address, port = lock_file.port, protocol = lock_file.protocol},
    ).header("authorization", format!{"Basic {auth}", auth = lock_file.b64_auth});
    Ok(request)
}

async fn get_summoner_id() -> Result<u64, Box<dyn Error>> {
    let lock_file = LeagueClientConnector::parse_lockfile()?;
    let request = Client::builder().danger_accept_invalid_certs(true).build()?
    .get(
        format! {"{protocol}://{ip}:{port}/lol-summoner/v1/current-summoner", ip = lock_file.address, port = lock_file.port, protocol = lock_file.protocol},
    ).header("authorization", format!{"Basic {auth}", auth = lock_file.b64_auth});
    let result: Value = request.send().await?.json().await?;
    let summoner_id = result.as_object().ok_or("Expecting object with summoner info")?["summonerId"].as_u64().ok_or("Expecting summoner Id")?;
    Ok(summoner_id)
}

async fn get_lobby_state(request: &RequestBuilder, summoner_id: u64) -> Result<LobbyState, Box<dyn Error>> {
    let response = request.try_clone().ok_or("Could not clone Request")?.send().await?;
    let base_json : Value = response.json().await?;
    let json = base_json.as_object().ok_or("Expecting object at top level")?;
    let bench = from_value(json["benchChampionIds"].clone())?;
    let mut team_champs = Vec::new();
    let mut your_champ = 0;
    for member_val in json["myTeam"].as_array().ok_or("Expecting list of team members")? {
        let member = member_val.as_object().ok_or("Expecting team member object")?;
        let champ_id = from_value(member["championId"].clone())?;
        if member["summonerId"].as_u64().ok_or("Expecting summoner Id of team member")? == summoner_id {
            your_champ = champ_id;
        } else {
            team_champs.push(champ_id);
        }
    }
    Ok(LobbyState{ your_champ, bench, team_champs})
}

fn get_model(tot_champs: usize) -> Result<Model, Box<dyn Error>> {
    let model = tract_onnx::onnx()
        // load the model
        .model_for_path("./model-trainer/model.onnx")?
        // specify input type and shape
        .with_input_fact(0, InferenceFact::dt_shape(f32::datum_type(), tvec![1, tot_champs + 1]))?
        // optimize graph
        .into_optimized()?
        // make the model runnable and fix its inputs and outputs
        .into_runnable()?;
    Ok(model)
}


#[tokio::main]
async fn main() {
    let champs: &Vec<(String, u16)> = &serde_json::from_reader(File::open("model-trainer/champs.json").unwrap()).unwrap();
    let tot_champs = champs.len();
    let champ_dict = &map_champ_id_to_index(champs).unwrap();
    let model = &get_model(tot_champs).unwrap();
    let summoner_id = get_summoner_id().await.unwrap();
    let request = &make_lobby_request().unwrap();
    loop {
        let lobby = get_lobby_state(request, summoner_id).await.unwrap();
        for champ in [lobby.your_champ].clone().into_iter().chain(lobby.bench.clone().into_iter()) {
            let team: [u16; 5] = [champ].into_iter().chain(lobby.team_champs.clone().into_iter()).collect::<Vec<_>>().try_into().unwrap();
            let win_rate = get_win_rate(&team, champ_dict, model).unwrap();
            println!{"{win_rate}"};
        }
    }
    //dioxus::desktop::launch(app)
}

fn app(mut cx: Scope) -> Element {
    let champs: &Vec<(String, u16)> = &serde_json::from_reader(File::open("model-trainer/champs.json").unwrap()).unwrap();
    let tot_champs = champs.len();
    let champ_dict = &map_champ_id_to_index(champs).unwrap();
    let model = &get_model(tot_champs).unwrap();
    let (count, set_count) = use_state(&mut cx, || 0);

    cx.render(rsx! {
        h1 { "Count: {count}" }
        button { onclick: move |_| set_count(count+1), "+" }
        button { onclick: move |_| set_count(count-1), "-" }
    })
}

