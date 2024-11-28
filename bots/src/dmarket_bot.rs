use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    bots::setup_env();
    start_dmarket().await?;
    Ok(())
}

async fn start_dmarket() -> Result<()> {
    todo!()
}
