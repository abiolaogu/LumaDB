mod config;
mod metrics;
mod config;
mod metrics;
mod server;
mod connection;

use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to config file
    #[arg(short, long, default_value = "config.toml")]
    config: PathBuf,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    let args = Args::parse();
    
    println!("Loading config from {:?}", args.config);
    let config = config::Config::load(args.config.to_str().unwrap())?;
    
    server::run(config).await
}
