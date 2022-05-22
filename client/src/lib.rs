use std::{
    collections::HashMap,
    fmt::{Debug},
    fs::File, path::Path,
};

use dioxus::{prelude::*, desktop::tao::dpi::LogicalSize};
use lol_client_api::{ChampSelectFetcher, RealChampSelectFetcher};
use serde_json;
use tokio::time::*;
use tract_onnx::prelude::*;
use anyhow::Error;
pub mod simulator;
mod lol_client_api;
mod models;

type Champ = u16;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChampSelectState {
    your_champ: Champ,
    bench: Vec<Champ>,
    team_champs: [Champ; 4],
}

struct Model {
    model: SimplePlan<TypedFact, Box<dyn TypedOp>, Graph<TypedFact, Box<dyn TypedOp>>>,
    champ_dict: HashMap<u16, usize>,
}

impl Model {
    fn get_win_rate(&self, team: &[u16; 5]) -> Result<f32, Error> {
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
) -> Result<HashMap<u16, usize>, Error> {
    let mut map = HashMap::new();
    for i in 0..all_champs.len() {
        map.insert(all_champs[i].1, i);
    }
    Ok(map)
}

fn get_model() -> Result<Model, Error> {
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

pub fn start_app() {
    dioxus::desktop::launch_cfg(App, |cfg| {
        cfg.with_window(|w| {
            w.with_title("ARAM champ select helper")
            .with_inner_size(LogicalSize::new(620.0, 120.0))
        })
    });
}

#[allow(non_snake_case)]
fn App(cx: Scope) -> Element {
    let champ_select_fetcher = use_future(&cx, (), |_| async {
        #[cfg(feature = "simulator")]
        {
            use simulator::FakeChampSelectFetcher;
            return FakeChampSelectFetcher::new().await;
        }
        #[cfg(not(feature = "simulator"))]
        return RealChampSelectFetcher::new().await;
    }).value();
    let fetcher = match champ_select_fetcher
    {
        Some(fetcher) => fetcher,
        None => return cx.render(rsx!("Waiting for lol client")),
    };
    cx.render(rsx!(ChampSelect {fetcher: fetcher.clone()}))
}

#[derive(Props)]
struct ChampSelectProps<Fetcher: ChampSelectFetcher> {
    fetcher: Fetcher
}

fn ok_ref<T, E>(res: &Result<T, E>) -> Option<&T> {
    match res {
        Ok(val) => Some(val),
        Err(_) => None,
    }
}

#[allow(non_snake_case)]
fn ChampSelect<Fetcher: ChampSelectFetcher + Clone + 'static>(cx: Scope<ChampSelectProps<Fetcher>>) -> Element {
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
