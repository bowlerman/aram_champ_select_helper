use std::{fmt::Debug, path::Path};

use dioxus::{desktop::tao::dpi::LogicalSize, prelude::*};
use lol_client_api::ChampSelectFetcher;
use models::aram::ARAMModel;
use tokio::time::*;
pub mod lol_client_api;
mod models;
pub mod simulator;

type Champ = u16;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChampSelectState {
    your_champ: Champ,
    bench: Vec<Champ>,
    team_champs: [Champ; 4],
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

pub fn start_app<Fetcher: ChampSelectFetcher + Clone + 'static>() {
    dioxus::desktop::launch_cfg(App::<Fetcher>, |cfg| {
        cfg.with_window(|w| {
            w.with_title("ARAM champ select helper")
                .with_inner_size(LogicalSize::new(620.0, 120.0))
        })
    });
}

#[allow(non_snake_case)]
fn App<Fetcher: ChampSelectFetcher + Clone + 'static>(cx: Scope) -> Element {
    {
        let champ_select_fetcher = use_future(&cx, (), |_| async {
            return Fetcher::new().await;
        })
        .value();
        let fetcher = match champ_select_fetcher {
            Some(fetcher) => fetcher,
            None => return cx.render(rsx!("Waiting for lol client")),
        };
        cx.render(rsx!(ChampSelect {
            fetcher: fetcher.clone()
        }))
    }
}

#[derive(Props)]
struct ChampSelectProps<Fetcher: ChampSelectFetcher> {
    fetcher: Fetcher,
}

fn ok_ref<T, E>(res: &Result<T, E>) -> Option<&T> {
    match res {
        Ok(val) => Some(val),
        Err(_) => None,
    }
}

#[allow(non_snake_case)]
fn ChampSelect<Fetcher: ChampSelectFetcher + Clone + 'static>(
    cx: Scope<ChampSelectProps<Fetcher>>,
) -> Element {
    let model = use_state(&cx, || ARAMModel::new().unwrap()); //application is useless without valid model
    let model = model.current();
    let fetcher = cx.props.fetcher.clone();
    let champ_select_state_handle: &UseState<Option<ChampSelectState>> = use_state(&cx, || None);
    let champ_select_state_store = champ_select_state_handle.clone();
    use_coroutine::<(), _, _>(&cx, |_| async move {
        let mut wait = interval(Duration::from_millis(1000));
        loop {
            let (maybe_champ_select_state, _) =
                tokio::join!(fetcher.get_champ_select_state(), wait.tick());
            if ok_ref(&maybe_champ_select_state)
                != champ_select_state_store.current().as_ref().as_ref()
            {
                champ_select_state_store.set(maybe_champ_select_state.ok())
            }
        }
    });
    let state = champ_select_state_handle.current().as_ref().clone();
    match state {
        None => cx.render(rsx!("Waiting for champ select")),
        Some(champ_select_state) => {
            let team_champs = champ_select_state.team_champs;
            let win_rates = champ_select_state
                .choices()
                .into_iter()
                .filter_map(|choice| {
                    Some((
                        choice,
                        model
                            .get_win_rate(&[
                                choice,
                                team_champs[0],
                                team_champs[1],
                                team_champs[2],
                                team_champs[3],
                            ])
                            .ok()?,
                    ))
                });
            let win_rate_displays = win_rates.map(|(choice, win_rate)| {
                rsx!(WinRateDisplay {
                    champ: choice,
                    win_rate: win_rate
                })
            });
            cx.render(rsx!(div {
                display: "flex",
                flex_wrap: "wrap",
                win_rate_displays
            }))
        }
    }
}

#[derive(Props, PartialEq)]
struct WinRateDisplayProps {
    champ: Champ,
    win_rate: f32,
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
