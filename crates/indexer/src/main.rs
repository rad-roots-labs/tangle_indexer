use anyhow::Result;
use clap::Parser;
use tracing::{error, info};

use radroots_market_relay_indexer::{cli, config::Settings, run, telemetry};

#[tokio::main]
async fn main() {
    if let Err(err) = setup().await {
        error!("Fatal error: {err:#?}");
        std::process::exit(1);
    }
}

async fn setup() -> Result<()> {
    let args = cli::Args::parse();

    let settings = Settings::load(&args.config)?;

    telemetry::init(&settings.service.logs_dir);
    info!("Service starting");

    run(settings).await
}
