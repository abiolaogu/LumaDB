use tokio_postgres::{NoTls, Error};
use std::io::{self, Write};

#[tokio::main]
async fn main() -> Result<(), Error> {
    println!("LumaDB Unified CLI v2.0.0");
    println!("Connecting to LumaDB on localhost:5432 (Postgres Protocol)...");

    // Connect to LumaDB using tokio-postgres
    // Note: LumaDB auth is mock, so any user/password works
    let (client, connection) = tokio_postgres::connect("host=localhost port=5432 user=admin", NoTls).await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    println!("Connected! Type 'exit' to quit.");

    loop {
        print!("luma> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let cmd = input.trim();

        if cmd == "exit" || cmd == "quit" {
            break;
        }

        if cmd.is_empty() {
            continue;
        }

        // Execute query
        // LumaDB PG adapter currently expects Simple Query protocol, which tokio-postgres uses for batch_execute
        // But for param queries it uses extended. 
        // Our adapter mainly supports Simple Query (Q).
        // Let's use simple_query
        
        match client.simple_query(cmd).await {
            Ok(messages) => {
                for msg in messages {
                    match msg {
                        tokio_postgres::SimpleQueryMessage::Row(row) => {
                           // Simple print of columns
                           for i in 0..row.len() {
                               if i > 0 { print!(" | "); }
                               print!("{}", row.get(i).unwrap_or("NULL"));
                           }
                           println!("");
                        },
                        tokio_postgres::SimpleQueryMessage::CommandComplete(tag) => {
                            println!("OK: {}", tag);
                        },
                        _ => {}
                    }
                }
            }
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        }
    }

    Ok(())
}
