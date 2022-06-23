use std::{fmt::Debug};

use anyhow::Error;
use iced::{pure::{Element, Application, widget::{Text, Column}}, Subscription, Command, executor};
use lol_client_api::ChampSelectFetcher;

use once_cell::sync::OnceCell;
use simulator::FakeChampSelectFetcher;

pub mod lol_client_api;
mod models;
pub mod simulator;

type Champ = u16;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ARAMChampSelectState {
    your_champ: Champ,
    bench: Vec<Champ>,
    team_champs: [Champ; 4],
}

impl ARAMChampSelectState {
    fn choices(&self) -> Vec<u16> {
        let mut ret = vec![self.your_champ];
        ret.extend(&self.bench);
        ret
    }
}

trait Dilemma<A: Application>
where A::Message: 'static {
    type Choice;

    fn choices(&self) -> Vec<Self::Choice>;

    fn eval(&self, choice: &Self::Choice) -> Result<f32, Error>;

    fn repr_choice(&'_ self, choice: Self::Choice) -> Element<'_, A::Message>;

    fn repr_choice_with_win(&'_ self, choice: Self::Choice) -> Element<'_, A::Message> {
        if let Ok(win_rate) = self.eval(&choice) {
            Column::new()
                .push(self.repr_choice(choice))
                .push(Text::new(format!("{:.2}%", win_rate * 100.0)))
                .into()
        }
        else {
            Column::new()
                .push(self.repr_choice(choice))
                .push(Text::new("Error"))
                .into()
        }
    }

    fn repr_choices_with_win(&'_ self) -> Element<'_, A::Message>
    {
        let mut col = Column::new();
        for choice in self.choices() {
            col = col.push(self.repr_choice_with_win(choice));
        }
        col.into()
    }
}

#[test]
fn test_choices() {
    let cs = ARAMChampSelectState {
        your_champ: 12,
        bench: vec![1, 51, 124, 12, 53],
        team_champs: [123, 12, 3, 1],
    };
    assert_eq!(cs.choices().len(), cs.bench.len() + 1)
}

struct ChampSelectProps<Fetcher: ChampSelectFetcher> {
    fetcher: Fetcher,
}

fn ok_ref<T, E>(res: &Result<T, E>) -> Option<&T> {
    match res {
        Ok(val) => Some(val),
        Err(_) => None,
    }
}
/*
#[allow(non_snake_case)]
fn ChampSelect<Fetcher: ChampSelectFetcher + Clone + 'static>(
    cx: Scope<ChampSelectProps<Fetcher>>,
) -> Element {
    let fetcher = cx.props.fetcher.clone();
    let champ_select_state_handle: &UseState<Option<ARAMChampSelectState>> = use_state(&cx, || None);
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
    let model = use_state(&cx, || ARAMAIModel::new().unwrap()); //application is useless without valid model
    let model = model.current();
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
                rsx!(ARAMWinRateDisplay {
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
*/

struct ARAMWinRateDisplayProps {
    champ: Champ,
    win_rate: f32,
}

struct ChampDisplayProps {
    champ: Champ
}
/*
#[allow(non_snake_case)]
fn ChampDisplay(cx: Scope<ChampDisplayProps>) -> Element {
    let champ = cx.props.champ;
    let image_path = if Path::new(&format!("client/champ_icons/{champ}.png")).exists() {
        format!("client/champ_icons/{champ}.png")
    } else {
        "client/champ_icons/generic.png".to_owned()
    };
    cx.render(rsx!(img {src: "{image_path}", width: "60", height: "60"}))
}

#[allow(non_snake_case)]
fn ARAMWinRateDisplay(cx: Scope<ARAMWinRateDisplayProps>) -> Element {
    let win_rate = cx.props.win_rate * 100_f32;
    let champ = cx.props.champ;
    cx.render(rsx!(div {display: "flex", flex_direction: "column", ChampDisplay{ champ: champ }, "{win_rate:.1} %"}))
}*/

#[derive(Debug, Default)]
pub struct App {
    champ_select_state: Option<ARAMChampSelectState>,
    champ_select_fetcher: OnceCell<FakeChampSelectFetcher>,
    error: Option<Error>,
    count: u32,
}

#[derive(Debug)]
pub enum Message {
    InitFetcher(FakeChampSelectFetcher),
    ChampSelectState(ARAMChampSelectState),
    Error(Error),
    Quit,
}

impl Application for App {
    type Executor = executor::Default;

    type Message = Message;

    type Flags = ();

    #[inline]
    fn new(_flags: Self::Flags) -> (Self, Command<Self::Message>) {
        let command = Command::perform(async {
            FakeChampSelectFetcher::new().await
        }, |fetcher| Message::InitFetcher(fetcher));
        (App::default(), command)
    }

    fn title(&self) -> String {
        "ARAM Assistant".into()
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::Quit => std::process::exit(0),
            Message::InitFetcher(fetcher) => {
                self.champ_select_fetcher.set(fetcher.clone()).expect("InitFetcher should only be sent once");
            },
            Message::ChampSelectState(state) => self.champ_select_state = Some(state),
            Message::Error(err) => {
                self.error = Some(err);
            }
        }
        self.count = self.count.wrapping_add(1);
        Command::none()
    }

    fn view(&self) -> Element<'_, Self::Message> {
        if let Some(fetcher) =  self.champ_select_fetcher.get() {
            if let Some(champ_select) = &self.champ_select_state {
                return Column::new()
                    .push(Text::new("Your team:"))
                    .push(Text::new(format!("{:?}", champ_select))).into();
            } else {

            Text::new(format!("Found LoL client {}", self.count)).into()}
        } else {
            Text::new("Waiting for LoL client").into()
        }
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        if let Some(fetcher) =  self.champ_select_fetcher.get() {
            Subscription::from_recipe(fetcher.clone())
        } else {
            Subscription::none()
        }
    }
}