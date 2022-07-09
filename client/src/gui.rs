use std::{path::Path, sync::Arc, time::Duration};

use anyhow::Error;
use client::models::aram::ARAMAIModel;
use iced::{
    executor,
    futures::FutureExt,
    pure::{
        widget::{Button, Column, Image, Row, Text, TextInput},
        Application, Element,
    },
    Command, Length, Settings, Subscription,
};
use iced_futures::backend::default::time;
use once_cell::sync::OnceCell;

use crate::logic::lol_client_api::ChampSelectFetcher;

use crate::logic::{ARAMChampSelectState, Champ};

const CHAMPICON_SIZE: Length = Length::Units(64);

fn view_champ(champ: Champ) -> Element<'static, Message> {
    let image_path = if Path::new(&format!("client/champ_icons/{}.png", champ.id)).exists() {
        format!("client/champ_icons/{}.png", champ.id)
    } else {
        "client/champ_icons/generic.png".to_owned()
    };
    Image::new(image_path)
        .height(CHAMPICON_SIZE)
        .width(CHAMPICON_SIZE)
        .into()
}

#[derive(Debug, Default)]
struct App {
    champ_select_state: ClientState,
    champ_select_fetcher: OnceCell<ChampSelectFetcher>,
    aram_ai_model: OnceCell<ARAMAIModel>,
    debug: DebugInfo,
    manual_input_enabled: bool,
    selected_champ: Champ,
    champ_input: String,
}

#[derive(Debug)]
enum ClientState {
    WaitingForChampSelect,
    ChampSelect(ARAMChampSelectState)
}

impl Default for ClientState {
    fn default() -> Self {
        ClientState::WaitingForChampSelect
    }
}

impl ClientState {
    fn is_champ_select(&self) -> bool {
        matches!(self, ClientState::ChampSelect(_))
    }
}
#[derive(Debug, Default)]
struct DebugInfo {
    enabled: bool,
    message_count: u32,
}

pub fn main() -> Result<(), iced::Error> {
    App::run(Settings::default())
}

#[derive(Debug, Clone)]
enum Message {
    InitFetcher(Box<ChampSelectFetcher>),
    InitARAMAIModel(Box<ARAMAIModel>),
    SetChampSelectState(ARAMChampSelectState),
    UpdateChampSelectState,
    EnableManualInput,
    ManualInput(ManualInputMessage),
    Debug(DebugMessage),
    Error(Arc<Error>),
}

#[derive(Debug, Clone)]
enum ManualInputMessage {
    ChangeSelection(String),
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
        let command_fetcher = Command::perform(
            ChampSelectFetcher::new().map(Box::new),
            Message::InitFetcher,
        );
        let command_ai_model = Command::perform(async { ARAMAIModel::new() }, |r| match r {
            Err(e) => Message::Error(Arc::new(e)),
            Ok(v) => Message::InitARAMAIModel(Box::new(v)),
        });
        (
            App::default(),
            Command::batch([command_fetcher, command_ai_model]),
        )
    }

    fn title(&self) -> String {
        "ARAM Assistant".into()
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        if self.debug.enabled {
            self.update_debug();
        }
        match message {
            Message::InitFetcher(fetcher) => {
                self.champ_select_fetcher
                    .set(*fetcher)
                    .expect("InitFetcher should only be sent once");
            }
            Message::InitARAMAIModel(model) => {
                self.aram_ai_model
                    .set(*model)
                    .expect("InitARAMAIModel should only be sent once");
            }
            Message::SetChampSelectState(state) => self.champ_select_state = ClientState::ChampSelect(state),
            Message::UpdateChampSelectState => {
                let fetcher = self.champ_select_fetcher.get().expect("Update requests should only be sent if there is a fetcher").clone();
                return Command::perform(
                    async move { fetcher.get_champ_select_state().await },
                    |r| match r {
                        Err(e) => Message::Error(Arc::new(e)),
                        Ok(v) => Message::SetChampSelectState(v),
                    },
                );
            }
            Message::Debug(debug) => match debug {
                DebugMessage::EnableDebug => self.debug.enabled = true,
            },
            Message::EnableManualInput => {
                self.manual_input_enabled = true;
                if let ClientState::WaitingForChampSelect = self.champ_select_state {
                    self.champ_select_state = ClientState::ChampSelect(Default::default());
                }
            }
            Message::ManualInput(input) => {
                self.update_manual(input);
            }
            Message::Error(_) => {} // TODO: Log/display error
        }

        Command::none()
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        if self.champ_select_fetcher.get().is_some() && !self.manual_input_enabled {
            time::every(Duration::from_millis(1000)).map(|_| Message::UpdateChampSelectState)
        } else {
            Subscription::none()
        }
    }

    fn view(&self) -> Element<Self::Message> {
        Column::new()
            .push(self.debug_view())
            .push(self.settings_view())
            .push(self.champ_select_container_view())
            .into()
    }
}

impl App {
    fn update_debug(&mut self) {
        self.debug.message_count = self.debug.message_count.wrapping_add(1);
    }

    fn update_manual(&mut self, input: ManualInputMessage) {
        if !self.champ_select_state.is_champ_select() {
            self.champ_select_state = ClientState::ChampSelect(Default::default());
        }
        let champ_select_state = match &mut self.champ_select_state {
            ClientState::ChampSelect(state) => state,
            _ => unreachable!("We just set client state to champ select"),
        };
        match input {
            ManualInputMessage::AddBench(champ) => {
                champ_select_state.add_bench(champ);
            }
            ManualInputMessage::RemoveBench(index) => {
                champ_select_state
                    .remove_bench(index);
            }
            ManualInputMessage::SwapBench(index, champ) => {
                champ_select_state
                    .swap_bench(index, champ);
            }
            ManualInputMessage::SwapTeam(index, champ) => {
                champ_select_state
                    .swap_team(index, champ);
            }
            ManualInputMessage::ChangeSelection(champ) => {
                self.champ_input = champ.clone();
                if let Ok(champ) = champ.try_into() {
                    self.selected_champ = champ
                }
            }
        }
        self.champ_select_state = ClientState::ChampSelect(champ_select_state.clone());
    }

    fn settings_view(&self) -> Element<Message> {
        let mut row = Row::new();
        self.debug_view();
        if !self.debug.enabled {
            row = row.push(
                Button::new("Enable debug mode")
                    .on_press(Message::Debug(DebugMessage::EnableDebug)),
            );
        }
        if !self.manual_input_enabled {
            row = row.push(Button::new("Enable manual input").on_press(Message::EnableManualInput));
        }
        row.into()
    }

    fn debug_view(&self) -> Element<Message> {
        let row = Row::new();
        if self.debug.enabled {
            row.push(Text::new(format!(
                "Debug mode enabled. message_count = {}",
                self.debug.message_count
            )))
        } else {
            row
        }
        .into()
    }

    fn champ_select_container_view(&self) -> Element<Message> {
        let mut col = Column::new();
        if let ClientState::ChampSelect(champ_select_state) = &self.champ_select_state {
            if self.manual_input_enabled {
                col = col.push(self.manual_input_view());
            }
            col = col
                .push(self.champ_select_view(champ_select_state))
                .push(self.win_rate_view(champ_select_state));
        } else {
            col = match self.manual_input_enabled {
                // We initialize champ select when enabling manual input, so this should never happen.
                true => col.push(Text::new("Error: No champ select state found.")),
                false => col.push(Text::new("Waiting for champ select")),
            };
        }
        col.into()
    }

    fn champ_select_view(&self, champ_select: &ARAMChampSelectState) -> Element<Message> {
        use ManualInputMessage::*;
        use Message::ManualInput;
        let mut row = Row::new();
        for (i, &champ) in champ_select.team_champs.iter().enumerate() {
            let button = Button::new(view_champ(champ))
                .on_press(ManualInput(SwapTeam(i, self.selected_champ)));
            row = row.push(button);
        }
        row = row.push(Text::new("Bench"));
        for (i, &champ) in champ_select.bench.iter().enumerate() {
            let button = Button::new(view_champ(champ))
                .on_press(ManualInput(SwapBench(i, self.selected_champ)));
            row = row.push(button);
        }
        row.push(
            Column::new()
                .push(
                    Button::new(Text::new("+"))
                        .on_press(ManualInput(AddBench(self.selected_champ))),
                )
                .push(
                    Button::new(Text::new("-"))
                        .on_press(ManualInput(RemoveBench(champ_select.bench.len() - 1))),
                ),
        )
        .into()
    }

    fn manual_input_view(&self) -> Element<Message> {
        let row = Row::new();
        row.push(TextInput::new("champion id", &self.champ_input, |c| {
            Message::ManualInput(ManualInputMessage::ChangeSelection(c))
        }))
        .into()
    }

    fn win_rate_view(&self, champ_select_state: &ARAMChampSelectState) -> Element<Message> {
        let mut row = Row::new();
        if let Some(model) = self.aram_ai_model.get() {
            for champ in champ_select_state.choices() {
                let mut team = champ_select_state.clone();
                *team.your_champ_mut() = champ;
                let win_rate = model.get_win_rate(&team.team_champs).unwrap_or(f32::NAN);
                row = row.push(Text::new(format!("{:?}: {}%", champ, win_rate)));
            }
        } else {
            row = row.push(Text::new("Could not load AI model. Please "));
        }
        row.into()
    }
}
