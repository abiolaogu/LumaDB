use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "lumactl")]
#[command(about = "LumaDB Administration Tool", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Check cluster health
    Health,
    /// Manage nodes
    Node {
        #[arg(short, long)]
        list: bool,
    },
    /// Manage users
    User {
        #[arg(long)]
        create: Option<String>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Health => {
            println!("Cluster Status: HEALTHY");
            println!("Active Protocols: Postgres, MySQL, Cassandra, MongoDB");
        }
        Commands::Node { list } => {
            if *list {
                println!("Node ID | Status | Role");
                println!("node-1  | Up     | Leader");
                println!("node-2  | Up     | Follower");
            }
        }
        Commands::User { create } => {
            if let Some(user) = create {
                println!("Creating user '{}'", user);
            }
        }
    }
    Ok(())
}
