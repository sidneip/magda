use magda_desktop::cassandra;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_env_filter("debug").init();

    let args: Vec<String> = std::env::args().collect();
    let host = args.get(1).map(|s| s.as_str()).unwrap_or("localhost");
    let port: u16 = args
        .get(2)
        .and_then(|p| p.parse().ok())
        .unwrap_or(9042);
    let username = args.get(3).map(|s| s.as_str());
    let password = args.get(4).map(|s| s.as_str());

    println!("Testing Cassandra connection to {}:{}", host, port);
    if username.is_some() {
        println!("Using authentication for user '{}'", username.unwrap());
    }

    match cassandra::create_session(host, port, username, password).await {
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
