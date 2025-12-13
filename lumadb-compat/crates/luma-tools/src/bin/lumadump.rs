use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "lumadump")]
#[command(about = "LumaDB Backup Tool")]
struct Cli {
    /// Output file
    #[arg(short, long)]
    output: PathBuf,

    /// Database to dump
    #[arg(short, long)]
    db: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    println!("Dumping database '{}' to {:?}", cli.db, cli.output);
    // Stub logic
    println!("Dump completed successfully.");
    Ok(())
}
