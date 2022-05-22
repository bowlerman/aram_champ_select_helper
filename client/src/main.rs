use client::start_app;
#[cfg(feature = "simulator")]
use client::api_mock::simulator;

#[tokio::main]
async fn main() {
    #[cfg(feature = "simulator")]
    tokio::spawn(simulator());
    start_app();
}