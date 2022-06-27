use std::{fmt::Debug, time::Duration, path::Path};

use anyhow::Error;
use iced::{pure::{Element, Application, widget::{Text, Column, Button, Image, Row}, Widget, image}, Subscription, Command, executor, Settings, Renderer, Length};
use iced_futures::backend::default::time;
use lol_client_api::ChampSelectFetcher;

use once_cell::sync::OnceCell;

pub mod lol_client_api;
mod models;


#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct Champ{id: u16}

impl From<u16> for Champ {
    fn from(id: u16) -> Self {
        Self { id }
    }
}

const CHAMPICON_SIZE: Length = Length::Units(64);

impl Champ {
    fn view(self) -> Element<'static, Message> {
        let image_path = if Path::new(&format!("client/champ_icons/{}.png", self.id)).exists() {
            format!("client/champ_icons/{}.png", self.id)
        } else {
            "client/champ_icons/generic.png".to_owned()
        };
        Image::new(image_path).height(CHAMPICON_SIZE).width(CHAMPICON_SIZE).into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ARAMChampSelectState {
    your_champ: usize,
    bench: Vec<Champ>,
    team_champs: [Champ; 5],
}

impl Default for ARAMChampSelectState {
    fn default() -> Self {
        Self { your_champ: Default::default(), bench: Default::default(), team_champs: Default::default() }
    }
}

impl ARAMChampSelectState {
    fn choices(&self) -> Vec<Champ> {
        let mut ret = vec![self.team_champs[self.your_champ]];
        ret.extend(&self.bench);
        ret
    }

    fn add_bench(&mut self, champ: Champ) {
        self.bench.push(champ);
    }

    fn remove_bench(&mut self, index: usize) {
        if index < self.bench.len() {
            self.bench.remove(index);
        }
    }

    fn swap_bench(&mut self, index: usize, champ: Champ) {
        if index < self.bench.len() {
            self.bench[index] = champ;
        }
    }

    fn swap_team(&mut self, index: usize, champ: Champ) {
        if index < self.team_champs.len() {
            self.team_champs[index] = champ;
        }
    }

    fn view(&self) -> Element<Message> {
        let mut row = Row::new();
        for (i, &champ) in self.team_champs.iter().enumerate() {
            let button = Button::new(champ.view())
                .on_press(Message::ManualInput(ManualInputMessage::SwapTeam(i, 1.into())));
            row = row.push(button);
        }
        row = row.push(Text::new("Bench"));
        for &champ in &self.bench {
            row = row.push(champ.view());
        }
        row.into()
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
        your_champ: 4,
        bench: vec![1, 51, 124, 12, 53].iter().map(|&x| x.into()).collect(),
        team_champs: [123, 12, 3, 1, 12].map(|x| x.into()),
    };
    assert_eq!(cs.choices().len(), cs.bench.len() + 1)
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

#[allow(non_snake_case)]
fn ARAMWinRateDisplay(champ: &Champ, win_rate: f32) -> Element<Message> {
    let win_rate = win_rate * 100_f32;
    Column::new().push(champ.view()).push(Text::new(format!("{:.2}%", win_rate))).into()
}

#[derive(Debug, Default)]
struct App {
    champ_select_state: Option<ARAMChampSelectState>,
    champ_select_fetcher: OnceCell<ChampSelectFetcher>,
    message_count: u32,
    manual_input_enabled: bool,
    debug_enabled: bool,
}

pub fn main() -> Result<(), iced::Error> {
    App::run(Settings::default())
}

#[derive(Debug, Clone)]
enum Message {
    InitFetcher(ChampSelectFetcher),
    SetChampSelectState(ARAMChampSelectState),
    UpdateChampSelectState,
    EnableManualInput,
    ManualInput(ManualInputMessage),
    Debug(DebugMessage)
}

#[derive(Debug, Clone)]
enum ManualInputMessage {
    AddBench(Champ),
    RemoveBench(usize),
    SwapBench(usize, Champ),
    SwapTeam(usize, Champ),
}

#[derive(Debug, Clone, Copy)]
enum DebugMessage {
    EnableDebug,
}

impl Application for App {
    type Executor = executor::Default;

    type Message = Message;

    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Self::Message>) {
        let command = Command::perform(
            ChampSelectFetcher::new(), |fetcher| Message::InitFetcher(fetcher));
        (App::default(), command)
    }

    fn title(&self) -> String {
        "ARAM Assistant".into()
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        if self.debug_enabled {self.message_count = self.message_count.wrapping_add(1);}
        match message {
            Message::InitFetcher(fetcher) => {
                self.champ_select_fetcher.set(fetcher.clone()).expect("InitFetcher should only be sent once");
            },
            Message::SetChampSelectState(state) => self.champ_select_state = Some(state),
            Message::UpdateChampSelectState => {
                let fetcher = self.champ_select_fetcher.get().unwrap().clone();
                return Command::perform(async move {fetcher.get_champ_select_state().await.unwrap()}, Message::SetChampSelectState)
            },
            Message::Debug(debug) => match debug {
                DebugMessage::EnableDebug => self.debug_enabled = true,
            }
            Message::EnableManualInput => {
                self.manual_input_enabled = true;
                if self.champ_select_state.is_none() {
                    self.champ_select_state = Some(Default::default());
                }
            },
            Message::ManualInput(input) => {
                match input {
                    ManualInputMessage::AddBench(champ) => {
                        self.champ_select_state.as_mut().unwrap().add_bench(champ);
                    },
                    ManualInputMessage::RemoveBench(index) => {
                        self.champ_select_state.as_mut().unwrap().remove_bench(index);
                    },
                    ManualInputMessage::SwapBench(index, champ) => {
                        self.champ_select_state.as_mut().unwrap().swap_bench(index, champ);
                    },
                    ManualInputMessage::SwapTeam(index, champ) => {
                        self.champ_select_state.as_mut().unwrap().swap_team(index, champ);
                    },
                }
            },
        }

        Command::none()
    }

    fn view(&self) -> Element<Self::Message> {
        Column::new().push(self.settings_view()).push(self.champ_select_view()).into()
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        if self.champ_select_fetcher.get().is_some() && !self.manual_input_enabled {
            time::every(Duration::from_millis(1000)).map(|_| Message::UpdateChampSelectState)
        } else {
            Subscription::none()
        }
    }
}

impl App {
    fn settings_view(&self) -> Element<Message> {
        let mut row = Row::new();
        if self.debug_enabled {
            row = row.push(Text::new(format!("Debug mode enabled. message_count = {}", self.message_count)));
        } else {
            let but = Button::new("Enable debug mode").on_press(Message::Debug(DebugMessage::EnableDebug));
            row = row.push(but);
        }
        if !self.manual_input_enabled {
            row = row.push(Button::new("Enable manual input").on_press(Message::EnableManualInput));
        }
        row.into()
    }

    fn champ_select_view(&self) -> Element<Message> {
        let mut col = Column::new();
        if self.champ_select_fetcher.get().is_some() || self.manual_input_enabled {
            if let Some(champ_select) = &self.champ_select_state {
                col = col.push(champ_select.view());
            } else {
                if self.manual_input_enabled {
                    col = col.push(Text::new("Error: No champ select state found."))
                } else {
                    col = col
                        .push(Text::new("Waiting for champ select"))
                }
            }
        } else {
            col = col.push(Text::new("Waiting for LoL client"));
        }
        col.into()
    }
}
