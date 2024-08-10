mod scheduler;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    api::setup_env();
    let ws = api::ws::WsClient::connect().await?;
    ws.start().await?;
    Ok(())
}
