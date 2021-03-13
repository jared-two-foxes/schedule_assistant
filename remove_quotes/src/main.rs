use anyhow;
use schedule_assistant;
use chrono::Utc;

fn main() -> anyhow::Result<()> {
    dotenv::dotenv().expect("Failed to read .env file");

    let now = Utc::now();
    schedule_assistant::remove_expired_quotes( now );

    Ok(())
}
