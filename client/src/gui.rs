use std::{path::Path, time::Duration};

use iced::{
    executor,
    pure::{
        widget::{Button, Column, Image, Row, Text, TextInput},
        Application, Element,
    },
    Command, Length, Settings, Subscription, futures::FutureExt,
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
    champ_select_state: Option<ARAMChampSelectState>,
    champ_select_fetcher: OnceCell<ChampSelectFetcher>,
    debug: DebugInfo,
    manual_input_enabled: bool,
    selected_champ: Champ,
    champ_input: String,
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
    SetChampSelectState(ARAMChampSelectState),
    UpdateChampSelectState,
    EnableManualInput,
    ManualInput(ManualInputMessage),
    Debug(DebugMessage),
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
        let command = Command::perform(ChampSelectFetcher::new().map(Box::new), Message::InitFetcher);
        (App::default(), command)
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
            Message::SetChampSelectState(state) => self.champ_select_state = Some(state),
            Message::UpdateChampSelectState => {
                let fetcher = self.champ_select_fetcher.get().unwrap().clone();
                return Command::perform(
                    async move { fetcher.get_champ_select_state().await.unwrap() },
                    Message::SetChampSelectState,
                );
            }
            Message::Debug(debug) => match debug {
                DebugMessage::EnableDebug => self.debug.enabled = true,
            },
            Message::EnableManualInput => {
                self.manual_input_enabled = true;
                if self.champ_select_state.is_none() {
                    self.champ_select_state = Some(Default::default());
                }
            }
            Message::ManualInput(input) => {
                self.update_manual(input);
            }
        }

        Command::none()
    }

    fn view(&self) -> Element<Self::Message> {
        Column::new()
            .push(self.debug_view())
            .push(self.settings_view())
            .push(self.champ_select_container_view())
            .into()
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
    fn update_debug(&mut self) {
        self.debug.message_count = self.debug.message_count.wrapping_add(1);
    }

    fn update_manual(&mut self, input: ManualInputMessage) {
        match input {
            ManualInputMessage::AddBench(champ) => {
                self.champ_select_state.as_mut().unwrap().add_bench(champ);
            }
            ManualInputMessage::RemoveBench(index) => {
                self.champ_select_state
                    .as_mut()
                    .unwrap()
                    .remove_bench(index);
            }
            ManualInputMessage::SwapBench(index, champ) => {
                self.champ_select_state
                    .as_mut()
                    .unwrap()
                    .swap_bench(index, champ);
            }
            ManualInputMessage::SwapTeam(index, champ) => {
                self.champ_select_state
                    .as_mut()
                    .unwrap()
                    .swap_team(index, champ);
            }
            ManualInputMessage::ChangeSelection(champ) => {
                self.champ_input = champ.clone();
                if let Ok(champ) = champ.try_into() {self.selected_champ = champ}
            },
        }
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
        if let Some(champ_select_state) = &self.champ_select_state {
            if self.manual_input_enabled {
                col = col.push(self.manual_input_view());
            }
            col = col.push(self.champ_select_view(champ_select_state)).push(self.win_rate_view(champ_select_state));
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
        row.push(TextInput::new("champion id", &self.champ_input, |c| Message::ManualInput(ManualInputMessage::ChangeSelection(c)))).into()
    }

    fn win_rate_view(&self, champ_select_state: &ARAMChampSelectState) -> Element<Message> {
        //let mut row = Row::new();
        todo!()
    }
}
