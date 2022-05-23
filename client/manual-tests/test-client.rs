use client::{simulator::{simulator, FakeChampSelectFetcher}, start_app};

#[tokio::main]
async fn main() {
    tokio::spawn(simulator());
    start_app::<FakeChampSelectFetcher>();
}