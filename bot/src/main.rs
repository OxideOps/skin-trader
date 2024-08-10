use api::ws::Channel;

mod scheduler;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    api::setup_env();
    let ws = api::WsClient::connect(|channel, data| {
        log::info!(
            "Message from server - Channel: {:?}, Data: {}",
            channel,
            data
        );
        Ok(())
    })
    .await?;
    ws.start().await
}
