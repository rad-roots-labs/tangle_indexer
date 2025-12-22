use std::path::Path;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling;
use tracing_subscriber::{fmt, prelude::*, EnvFilter, Registry};

#[cfg(feature = "audit")]
use tracing_subscriber::filter::Targets;

pub struct TelemetryGuards {
    _file: WorkerGuard,
    _audit: Option<WorkerGuard>,
}

pub fn init(logs_dir: impl AsRef<Path>) -> TelemetryGuards {
    let logs_path = logs_dir.as_ref();
    if let Err(e) = std::fs::create_dir_all(logs_path) {
        eprintln!("Failed to create logs directory {}: {}", logs_path.display(), e);
    }

    let file_appender = rolling::daily(logs_path, concat!(env!("CARGO_PKG_NAME"), ".log"));
    let (file_writer, guard) = tracing_appender::non_blocking(file_appender);

    let stdout_layer = fmt::layer().with_writer(std::io::stdout).with_target(false);

    let file_layer = fmt::layer()
        .with_writer(file_writer)
        .with_ansi(false)
        .with_target(false);

    let subscriber = Registry::default()
        .with(EnvFilter::from_default_env())
        .with(stdout_layer)
        .with(file_layer);

    #[cfg(feature = "audit")]
    let (subscriber, audit_guard) = {
        let audit_app = rolling::daily(logs_path, "audit.log");
        let (audit_writer, guard) = tracing_appender::non_blocking(audit_app);

        let audit_layer = fmt::layer()
            .with_writer(audit_writer)
            .with_ansi(false)
            .with_target(true)
            .with_filter(Targets::new().with_target("audit", tracing::Level::INFO));

        (subscriber.with(audit_layer), Some(guard))
    };

    #[cfg(not(feature = "audit"))]
    let (subscriber, audit_guard) = (subscriber, None);

    subscriber.init();
    TelemetryGuards {
        _file: guard,
        _audit: audit_guard,
    }
}
