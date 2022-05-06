use std::{
    collections::HashMap,
    error::Error,
    fmt::{Debug},
    fs::File, path::Path,
};

use dioxus::{prelude::*, desktop::tao::dpi::LogicalSize};
#[cfg(not(feature = "simulator"))]
use league_client_connector::LeagueClientConnector;
#[cfg(not(feature = "simulator"))]
use reqwest::{Client, RequestBuilder};
use serde_json::{self, from_value, Value};
use tokio::time::*;
use tract_onnx::prelude::*;

type Champ = u16;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ChampSelectState {
    your_champ: Champ,
    bench: Vec<Champ>,
    team_champs: [Champ; 4],
}

struct Model {
    model: SimplePlan<TypedFact, Box<dyn TypedOp>, Graph<TypedFact, Box<dyn TypedOp>>>,
    champ_dict: HashMap<u16, usize>,
}

impl Model {
    fn get_win_rate(&self, team: &[u16; 5]) -> Result<f32, Box<dyn Error>> {
        let tot_champs = self.champ_dict.len();
        let mut one_hot = vec![0_f32; tot_champs + 1];
        for champ in team {
            one_hot[self.champ_dict.get(champ).cloned().unwrap_or(tot_champs)] = 1.;
        }
        let input = tract_ndarray::arr1(&one_hot).into_shape((1, tot_champs + 1))?;
        let tensor_res = self
            .model
            .run(tvec![input.into()])?
            .to_vec()
            .pop()
            .expect("Expecting model output");
        let res: [f32; 2] = tensor_res.as_slice()?.try_into()?;
        let sum = res[0] + res[1];
        Ok(res[0] / sum)
    }
}

impl ChampSelectState {
    fn choices(&self) -> Vec<u16> {
        let mut ret = vec![self.your_champ];
        ret.extend(&self.bench);
        ret
    }
}

#[test]
fn test_choices() {
    let cs = ChampSelectState {
        your_champ: 12,
        bench: vec![1, 51, 124, 12, 53],
        team_champs: [123, 12, 3, 1],
    };
    assert_eq!(cs.choices().len(), cs.bench.len() + 1)
}

fn map_champ_id_to_index(
    all_champs: &Vec<(String, u16)>,
) -> Result<HashMap<u16, usize>, Box<dyn Error>> {
    let mut map = HashMap::new();
    for i in 0..all_champs.len() {
        map.insert(all_champs[i].1, i);
    }
    Ok(map)
}

#[cfg(feature = "simulator")]
mod api_mock {

    use std::{sync::Mutex, convert::Infallible, io::stdin};

    use serde_json::{Value, json};
    use lazy_static::lazy_static;
    use crate::{ChampSelectState, Champ};
    use clap::{Parser, Subcommand, AppSettings};

    lazy_static! {
        static ref CHAMP_SELECT_STATE: Mutex<ChampSelectState> = Mutex::new(Default::default());
    }
    const SUMMONER_ID: u64 = 123;
    #[derive(Debug, Clone)]
    pub struct RequestBuilder {pub addr: String}

    pub struct Response {response: Value}

    impl RequestBuilder {
        pub async fn send(self) -> Result<Response, reqwest::Error> {
            let response = match self.addr.as_str() {
                "lol-champ-select/v1/session" => {
                    let champ_select_state = CHAMP_SELECT_STATE.lock().unwrap();
                    let mut team_champs: Vec<Value> = champ_select_state.team_champs.iter().map(|champ| json!({
                        "championId": champ,
                        "summonerId": SUMMONER_ID + 1
                    })).collect();
                    team_champs.push(json!({
                        "championId": champ_select_state.your_champ,
                        "summonerId": SUMMONER_ID
                    }));
                    json!(
                    {
                        "benchChampionIds": champ_select_state.bench,
                        "myTeam": team_champs
                    }
                )},
                "lol-summoner/v1/current-summoner" => json!({"summonerId": SUMMONER_ID}),
                _ => unimplemented!()
            };
            return Ok(Response{response})
        }

        pub fn try_clone(&self) -> Option<RequestBuilder> {
            Some(self.clone())
        }
    }

    impl Response {
        pub async fn json(&self) -> Result<Value, Infallible> {
            return Ok(self.response.clone())
        }
    }


    #[derive(Parser, Debug)]
    #[clap(global_setting(AppSettings::NoBinaryName))]
    struct Cli{
        #[clap(subcommand)]
        command: Commands
    }

    #[derive(Subcommand, Debug)]
    enum Commands{
        AddBench {champ: Champ},
        RmBench,
        YourChamp {champ: Champ},
        TeamChamps {pos: usize, champ: Champ},
        Print,
    }

    use Commands::*;

    pub async fn simulator() {
        loop {
            let mut buffer = String::new();
            stdin().read_line(&mut buffer).unwrap();
            let maybe_cli = Cli::try_parse_from(buffer.split_whitespace());
            let cli = match maybe_cli {
                Ok(cli) => cli,
                Err(err) => {err.print().unwrap(); continue}
            };
            let mut champ_select_state = CHAMP_SELECT_STATE.lock().unwrap();
            match cli.command {
                AddBench{champ} => {
                    champ_select_state.bench.push(champ)
                },
                RmBench => {
                    champ_select_state.bench.pop();
                },
                YourChamp{champ} => {
                    champ_select_state.your_champ = champ
                },
                TeamChamps{pos, champ} => {
                    champ_select_state.team_champs[pos] = champ
                },
                Print => ()
            }
            println!("{champ_select_state:?}");
        }
    }
}

#[cfg(feature = "simulator")]
use api_mock::*;

fn make_lol_client_api_request(addr: &str) -> Result<RequestBuilder, Box<dyn Error>> {
    #[cfg(feature = "simulator")]{ // During debug return mock for client api
        Ok(RequestBuilder{addr: addr.to_owned()})
    }
    #[cfg(not(feature = "simulator"))]{
        let lock_file = LeagueClientConnector::parse_lockfile()?;
        let request = Client::builder().danger_accept_invalid_certs(true).build()?
        .get(
            format! {"{protocol}://{ip}:{port}/{addr}", ip = lock_file.address, port = lock_file.port, protocol = lock_file.protocol, addr = addr},
        ).header("authorization", format!{"Basic {auth}", auth = lock_file.b64_auth});
        Ok(request)
    }
}

fn make_lobby_request() -> Result<RequestBuilder, Box<dyn Error>> {
    make_lol_client_api_request("lol-champ-select/v1/session")
}

async fn try_get_summoner_id() -> Result<u64, Box<dyn Error>> {
    let result: Value = make_lol_client_api_request("lol-summoner/v1/current-summoner")?.send().await?.json().await?;
    let summoner_id = result
        .as_object()
        .ok_or("Expecting object with summoner info")?
        .get("summonerId")
        .ok_or("Expecting summonerId field")?
        .as_u64()
        .ok_or("Expecting summoner Id")?;
    Ok(summoner_id)
}

#[derive(Debug)]
struct ChampSelectFetcher {
    request: RequestBuilder,
    summoner_id: u64
}

impl Clone for ChampSelectFetcher {
    fn clone(&self) -> Self {
        Self { request: self.request.try_clone().unwrap(), summoner_id: self.summoner_id.clone() }
    }
}

impl ChampSelectFetcher {
    async fn get_champ_select_state(
        &self,
    ) -> Result<ChampSelectState, Box<dyn Error>>
    where
        dyn Error: 'static,
    {
        let response = self.request.try_clone().ok_or("Could not clone champ select api request")?.send().await?;
        let base_json: Value = response.json().await?;
        let json = base_json
            .as_object()
            .ok_or("Expecting object at top level")?;
        if let Some(v) = json.get("httpStatus")  {
            if let Value::Number(_) = v {
                Err("Not in champ select")?;
                unreachable!()
            }
        }
        let bench = from_value(json["benchChampionIds"].clone())?;
        let mut team_champs = Vec::new();
        let mut your_champ = 0;
        for member_val in json["myTeam"]
            .as_array()
            .ok_or("Expecting list of team members")?
        {
            let member = member_val
                .as_object()
                .ok_or("Expecting team member object")?;
            let champ_id = from_value(member["championId"].clone())?;
            if member["summonerId"]
                .as_u64()
                .ok_or("Expecting summoner Id of team member")?
                == self.summoner_id
            {
                your_champ = champ_id;
            } else {
                team_champs.push(champ_id);
            }
        }
        let team_champs = team_champs.as_slice().try_into()?;
        Ok(ChampSelectState {
            your_champ,
            bench,
            team_champs,
        })
    }
}

fn get_model() -> Result<Model, Box<dyn Error>> {
    let champs: Vec<(String, u16)> =
        serde_json::from_reader(File::open("model-trainer/champs.json").unwrap()).unwrap();
    let tot_champs = champs.len();
    let champ_dict = map_champ_id_to_index(&champs).unwrap();
    let model = tract_onnx::onnx()
        // load the model
        .model_for_path("./model-trainer/model.onnx")?
        // specify input type and shape
        .with_input_fact(
            0,
            InferenceFact::dt_shape(f32::datum_type(), tvec![1, tot_champs + 1]),
        )?
        // optimize graph
        .into_optimized()?
        // make the model runnable and fix its inputs and outputs
        .into_runnable()?;
    Ok(Model { model, champ_dict })
}

#[tokio::main]
async fn main() {
    #[cfg(feature = "simulator")]
    tokio::spawn(simulator());
    dioxus::desktop::launch_cfg(App, |cfg| {
        cfg.with_window(|w| {
            w.with_title("ARAM champ select helper")
            .with_inner_size(LogicalSize::new(620.0, 120.0))
        })
    });
}

async fn init_champ_select_fetcher() -> ChampSelectFetcher {
    let mut wait = interval(Duration::from_millis(1000));
    let summoner_id = loop {
        if let (Ok(summoner_id), _) = tokio::join!(try_get_summoner_id(), wait.tick()) {
            break summoner_id;
        }
    };
    let request = loop {
        if let Ok(req) = make_lobby_request() {
            break req;
        }
        wait.tick().await;
    };
    ChampSelectFetcher{request, summoner_id}
}

#[allow(non_snake_case)]
fn App(cx: Scope) -> Element {
    let champ_select_fetcher = use_future(&cx, (), |_| async {
        init_champ_select_fetcher().await
    }).value();
    let fetcher = match champ_select_fetcher
    {
        Some(fetcher) => fetcher,
        None => return cx.render(rsx!("Waiting for lol client")),
    };
    cx.render(rsx!(ChampSelect {fetcher: fetcher}))
}

#[derive(Props)]
struct ChampSelectProps<'a> {
    fetcher: &'a ChampSelectFetcher
}

fn ok_ref<T, E>(res: &Result<T, E>) -> Option<&T> {
    match res {
        Ok(val) => Some(val),
        Err(_) => None,
    }
}

#[allow(non_snake_case)]
fn ChampSelect<'a>(cx: Scope<'a, ChampSelectProps<'a>>) -> Element {
    let model = use_state(&cx, || get_model().unwrap()); //application is useless without valid model
    let model = model.current();
    let fetcher = cx.props.fetcher.clone();
    let champ_select_state_handle: &UseState<Option<ChampSelectState>> = use_state(&cx, || None);
    let champ_select_state_store = champ_select_state_handle.clone();
    use_coroutine::<(),_,_>(&cx, |_| async move {
        let mut wait = interval(Duration::from_millis(1000));
        loop {
            let (maybe_champ_select_state, _) = tokio::join!(fetcher.get_champ_select_state(), wait.tick());
            if ok_ref(&maybe_champ_select_state) != champ_select_state_store.current().as_ref().as_ref() {
                champ_select_state_store.set(maybe_champ_select_state.ok())
            }
        };
    });
    let state = champ_select_state_handle.current().as_ref().clone();
    match state {
        None => cx.render(rsx!("Waiting for champ select")),
        Some(champ_select_state) => {
            let team_champs = champ_select_state.team_champs;
            let win_rates = champ_select_state.choices().into_iter().filter_map(|choice| {
                Some((choice, model.get_win_rate(&[choice, team_champs[0], team_champs[1], team_champs[2], team_champs[3]]).ok()?))
            });
            let win_rate_displays = win_rates.map(|(choice, win_rate)| rsx!( WinRateDisplay {champ: choice, win_rate: win_rate} ));
            cx.render(rsx!( div {display: "flex", flex_wrap: "wrap", win_rate_displays }))
        },
    }
}

#[derive(Props, PartialEq)]
struct WinRateDisplayProps {
    champ: Champ,
    win_rate: f32
}

#[allow(non_snake_case)]
fn WinRateDisplay(cx: Scope<WinRateDisplayProps>) -> Element {
    let win_rate = cx.props.win_rate * 100_f32;
    let champ = cx.props.champ;
    let image_path = if Path::new(&format!("client/champ_icons/{champ}.png")).exists() {
        format!("client/champ_icons/{champ}.png")
    } else {
        "client/champ_icons/generic.png".to_owned()
    };
    cx.render(rsx!(div {display: "flex", flex_direction: "column", img {src: "{image_path}", width: "60", height: "60"}, "{win_rate:.1} %"}))
}
