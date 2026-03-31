mod auth;
mod config;
mod domain;
mod error;
mod io;
mod providers;
mod seed;
mod services;
mod state;
mod util;

use std::sync::Arc;

use tokio::net::TcpListener;

use crate::config::Config;
use crate::io::http::server::build_router;
use crate::providers::database::Database;
use crate::providers::timeseries::TimeSeries;
use crate::providers::{
    get_cache_provider, get_database_provider, get_logging_provider, get_storage_provider,
    get_timeseries_provider,
};
use crate::services::AppServices;
use crate::state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::load()?;
    let _logging = get_logging_provider(&config.logging)?;
    tracing::info!(
        mode = %config.application.mode.as_str(),
        "starting SocialLink API"
    );

    let database = Arc::new(get_database_provider(&config.database).await?);
    database.bootstrap().await?;
    let storage = Arc::new(get_storage_provider(&config.storage).await?);
    let timeseries = Arc::new(get_timeseries_provider(&config.timeseries).await?);
    timeseries.bootstrap().await?;
    let cache = Arc::new(get_cache_provider(&config.cache).await?);
    let services = Arc::new(AppServices::new(
        database, storage, timeseries, cache, &config,
    )?);

    let state = AppState {
        services,
        config: Arc::new(config),
    };
    seed::run(state.services.as_ref(), state.config.as_ref()).await?;

    let addr = format!("{}:{}", state.config.server.host, state.config.server.port);
    let listener = TcpListener::bind(&addr).await?;
    tracing::info!(%addr, "listening");

    let app = build_router(state);
    axum::serve(listener, app).await?;
    Ok(())
}
