use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    common::setup_env();
    start_dmarket().await?;
    Ok(())
}

async fn start_dmarket() -> Result<()> {
    let updater = dmarket::Updater::new().await?;
    updater.sync().await?;
    Ok(())
}
