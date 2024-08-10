mod plotter;
mod util;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    api::setup_env();
    let _db = api::db::Database::new().await?;
    Ok(())
}
