//! LumaDB CLI - Uses HTTP API instead of PostgreSQL protocol
//!
//! Since tokio_postgres is disabled, this CLI uses the HTTP API endpoint.

use std::io::{self, Write};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("LumaDB Unified CLI v2.0.0");
    println!("Using HTTP API on localhost:8123");
    println!("Type 'exit' to quit, or enter SQL/LQL queries.\n");

    loop {
        print!("luma> ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let cmd = input.trim();

        if cmd == "exit" || cmd == "quit" {
            break;
        }

        if cmd.is_empty() {
            continue;
        }

        // Send query via HTTP POST to ClickHouse-compatible endpoint
        // LumaDB HTTP API listens on 8123
        println!("Executing: {}", cmd);
        println!("(HTTP client not implemented - placeholder CLI)");
        println!("To use full functionality, enable tokio-postgres in Cargo.toml");
    }

    Ok(())
}
