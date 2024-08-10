use api::ws::Channel;

mod scheduler;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    api::setup_env();
    let db = api::Database::new().await?;
    let ws = api::WsClient::connect(|data| {
        // log::info!("Received channel: {:?}", data);
        Ok(())
    })
    .await?;
    ws.start().await
}
