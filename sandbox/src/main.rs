mod plotter;
mod util;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    api::setup_env();
    let db = api::Database::new().await?;
    let http = api::HttpClient::new();
    
    Ok(())
}
