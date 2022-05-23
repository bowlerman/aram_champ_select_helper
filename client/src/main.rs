use client::{start_app, lol_client_api::RealChampSelectFetcher};

#[tokio::main]
async fn main() {
    start_app::<RealChampSelectFetcher>();
}