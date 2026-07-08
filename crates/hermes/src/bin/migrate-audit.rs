use hermes::audit::migrate_sqlite_to_postgres;
use std::env;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sqlite_path = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("data/hipaa_hermes.db"));
    let database_url = env::args().nth(2).or_else(|| env::var("DATABASE_URL").ok()).ok_or(
        "usage: migrate-audit [sqlite_path] [database_url]\n  or set DATABASE_URL",
    )?;

    let migrated = migrate_sqlite_to_postgres(&sqlite_path, &database_url).await?;
    println!(
        "Migrated {migrated} audit entries from {} to Postgres",
        sqlite_path.display()
    );
    Ok(())
}
