use client::{lol_client_api::RealChampSelectFetcher, start_app};

#[tokio::main]
async fn main() {
    start_app::<RealChampSelectFetcher>();
}
