use clap::Parser;

#[derive(Parser)]
#[command(name = "lumapool")]
#[command(about = "LumaDB Connection Pool Manager")]
struct Cli {
    /// Port to listen on
    #[arg(short, long, default_value_t = 6432)]
    port: u16,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    println!("Starting connection pooler on port {}", cli.port);
    // Stub logic
    Ok(())
}
