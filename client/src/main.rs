use client::start_app;
#[cfg(feature = "simulator")]
use client::simulator::simulator;

#[tokio::main]
async fn main() {
    #[cfg(feature = "simulator")]
    tokio::spawn(simulator());
    start_app();
}