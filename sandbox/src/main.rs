mod plotter;
mod util;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let ws = api::ws::WsClient::connect().await?;
    ws.start().await?;
    Ok(())
}
