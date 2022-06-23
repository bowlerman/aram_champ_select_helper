use client::{simulator::simulator, App};
use iced::{pure::Application, Settings};

#[tokio::main]
async fn main() -> Result<(), iced::Error> {
    tokio::spawn(simulator());
    App::run(Settings::default())
}