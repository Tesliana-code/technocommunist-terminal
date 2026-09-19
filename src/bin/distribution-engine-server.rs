use std::{env, error::Error};

use distribution_machine::{
    persistence::DraftStore,
    server::{AppState, router},
};
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let database_url = env::var("DATABASE_URL").map_err(|_| "DATABASE_URL must be set")?;

    let bind =
        env::var("DISTRIBUTION_ENGINE_BIND").unwrap_or_else(|_| "127.0.0.1:18080".to_string());

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    let store = DraftStore::new(pool);
    store.ping().await?;

    let listener = tokio::net::TcpListener::bind(&bind).await?;

    println!("distribution-engine-server listening on http://{bind}");

    axum::serve(listener, router(AppState { store })).await?;

    Ok(())
}
