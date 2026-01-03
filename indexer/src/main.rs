use anyhow::Result;
use clap::Parser;
use radroots_radroots_indexer::{cli, run, telemetry, Settings};
use tracing::info;

#[tokio::main]
async fn main() {
    if let Err(err) = setup().await {
        eprintln!("Fatal error: {err:#?}");
        std::process::exit(1);
    }
}

async fn setup() -> Result<()> {
    let args = cli::Args::parse();

    let settings = Settings::load(&args.config)?;

    let _telemetry_guards = telemetry::init(&settings.indexer.logs_dir);
    info!("Service starting");

    run(settings).await
}
