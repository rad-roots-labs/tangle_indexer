use clap::Parser;

#[derive(Parser)]
#[command(
    about = env!("CARGO_PKG_DESCRIPTION"),
    author = env!("CARGO_PKG_AUTHORS"),
    version = env!("CARGO_PKG_VERSION")
)]
pub struct Args {
    #[arg(long, help = "(Optional) Defaults to 'config.toml'", required = false)]
    pub config: Option<String>,
}
