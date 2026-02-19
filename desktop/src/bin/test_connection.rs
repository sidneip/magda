use magda_desktop::cassandra;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_env_filter("debug").init();

    let host = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "localhost".to_string());
    let port: u16 = std::env::args()
        .nth(2)
        .and_then(|p| p.parse().ok())
        .unwrap_or(9042);

    println!("Testing Cassandra connection to {}:{}", host, port);

    match cassandra::create_session(&host, port).await {
        Ok(session) => {
            println!("Connected successfully!");

            if let Err(e) = cassandra::test_connection(&session).await {
                println!("Connection test failed: {}", e);
                return;
            }

            match cassandra::list_keyspaces(&session).await {
                Ok(keyspaces) => {
                    println!("Keyspaces: {:?}", keyspaces);
                    for ks in &keyspaces {
                        if !ks.starts_with("system") {
                            match cassandra::list_tables(&session, ks).await {
                                Ok(tables) => println!("  {}: {:?}", ks, tables),
                                Err(e) => println!("  {}: failed to list tables: {}", ks, e),
                            }
                        }
                    }
                }
                Err(e) => println!("Failed to list keyspaces: {}", e),
            }
        }
        Err(e) => println!("Failed to connect: {}", e),
    }
}
