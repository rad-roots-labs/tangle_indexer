use anyhow::{Context, Result};
use clap::Parser;
use radroots_market_relay_indexer::config::Settings;
use std::time::{Duration, Instant};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

fn init_tracing() {
    tracing_subscriber::fmt::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
}

#[derive(Parser)]
#[command(
    about = env!("CARGO_PKG_DESCRIPTION"),
    author = env!("CARGO_PKG_AUTHORS"),
    version = env!("CARGO_PKG_VERSION")
)]
pub struct CliArgs {
    #[arg(long, help = "(Optional) Defaults to 'config.toml'", required = false)]
    pub config: Option<String>,
}

#[tokio::main]
async fn main() {
    init_tracing();
    let cli_args = CliArgs::parse();

    if let Err(err) = run_service(cli_args).await {
        error!("Fatal error: {err:#?}");
        std::process::exit(1);
    }
}

async fn run_service(cli_args: CliArgs) -> Result<()> {
    info!("Service starting");

    let settings = Settings::load(&cli_args.config)
        .with_context(|| format!("Failed to load configuration from {:?}", cli_args.config))?;

    loop {
        let iteration_start = Instant::now();

        // loop end
        let elapsed = iteration_start.elapsed();
        let interval = Duration::from_secs(settings.service.flush_interval);
        let delay = interval.saturating_sub(elapsed);

        info!(
            elapsed_ms = elapsed.as_millis(),
            sleeping_ms = delay.as_millis(),
            "Iteration complete"
        );

        tokio::time::sleep(delay).await;
    }

    // unreachable
    #[allow(unreachable_code)]
    Ok(())
}
