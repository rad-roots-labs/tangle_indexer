use std::path::Path;
use tracing_appender::rolling;
use tracing_subscriber::{fmt, prelude::*, registry::Registry, EnvFilter};

pub fn init(logs_dir: impl AsRef<Path>) {
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
