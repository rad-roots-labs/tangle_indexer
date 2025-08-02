use anyhow::{Context, Result};
use clap::Parser;
use radroots_market_relay_indexer::config::Settings;
use std::path::Path;
use std::time::{Duration, Instant};
use tracing::{error, info};
use tracing_appender::rolling;
use tracing_subscriber::{fmt, prelude::*, registry::Registry, EnvFilter};

pub fn init_tracing(logs_dir: impl AsRef<Path>) {
    let file_appender = rolling::daily(logs_dir, concat!(env!("CARGO_PKG_NAME"), ".log"));
    let (file_writer, guard) = tracing_appender::non_blocking(file_appender);
    std::mem::forget(guard);

    let stdout_layer = fmt::layer().with_writer(std::io::stdout).with_target(false);

    let file_layer = fmt::layer()
        .with_writer(file_writer)
        .with_ansi(false)
        .with_target(false);

    Registry::default()
        .with(EnvFilter::from_default_env())
        .with(stdout_layer)
        .with(file_layer)
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
    let cli_args = CliArgs::parse();

    if let Err(err) = run_service(cli_args).await {
        error!("Fatal error: {err:#?}");
        std::process::exit(1);
    }
}

async fn run_service(cli_args: CliArgs) -> Result<()> {
    let settings = Settings::load(&cli_args.config)
        .with_context(|| format!("Failed to load configuration from {:?}", cli_args.config))?;

    init_tracing(settings.service.logs_dir);
    info!("Service starting");

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
