use client::{lol_client_api::{RealChampSelectFetcher, ChampSelectFetcher}, start_app, ARAMChampSelectState};
use iced::{pure::{Sandbox, Element, widget::{Text, Column, Button}}, executor, Command, Settings, futures::executor::block_on};

#[derive(Debug, Clone, Default)]
struct App {
    champ_select_state: Option<ARAMChampSelectState>,
    champ_select_fetcher: Option<RealChampSelectFetcher>,
}

#[derive(Debug, Clone)]
enum Message {
    Quit,
    AquireFetcher,
}

impl Sandbox for App {
    type Message = Message;

    #[inline]
    fn new() -> Self {
        App::default()
    }

    fn title(&self) -> String {
        "ARAM Assistant".into()
    }

    fn update(&mut self, message: Self::Message) {
        panic!("HI");
        match message {
            Message::Quit => std::process::exit(0),
            Message::AquireFetcher => {
                self.champ_select_fetcher = Some(block_on(RealChampSelectFetcher::new()));
            }
        }
    }

    fn view(&self) -> Element<'_, Self::Message> {
        Column::new().push(
        match self.champ_select_state.clone() {
            Some(champ_select_state) => {
                Text::new(format!("{:?}", champ_select_state))},
            None => Text::new("Loading...").into(),
        }).push(Button::new("Reload App").on_press(Message::AquireFetcher)).into()
    }
}

fn main() -> Result<(), iced::Error> {
    panic!("HELLO");
    App::run(Settings::default())
}
/*
#[tokio::main]
async fn main() {
    start_app::<RealChampSelectFetcher>();
}
 */