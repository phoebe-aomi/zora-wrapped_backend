#[tokio::main]
async fn main() {
    zora_aomi_tools::server::run_from_env().await.unwrap();
}
