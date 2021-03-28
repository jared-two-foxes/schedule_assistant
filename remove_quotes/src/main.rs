use anyhow;
use schedule_assistant;
use chrono::Utc;

fn main() -> anyhow::Result<()> {
    dotenv::dotenv().expect("Failed to read .env file");

    let now = Utc::now();
    let auth_cache = schedule_assistant::authentication::AuthenticationCache::new();
    schedule_assistant::remove_expired_quotes( &auth_cache, now );

    Ok(())
}
