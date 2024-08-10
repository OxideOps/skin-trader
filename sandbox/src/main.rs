mod plotter;
mod util;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _db = api::db::Database::new().await?;
    Ok(())
}
