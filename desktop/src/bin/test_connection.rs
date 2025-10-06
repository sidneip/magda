use magda_desktop::cassandra;

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("debug")
        .init();
    
    println!("🚀 Testing Cassandra connection to localhost:9042");
    
    match cassandra::create_session("localhost", 9042).await {
        Ok(session) => {
            println!("✅ Connected successfully!");
            
            // Test connection
            if let Err(e) = cassandra::test_connection(&session).await {
                println!("❌ Connection test failed: {}", e);
            }
            
            // List keyspaces
            match cassandra::list_keyspaces(&session).await {
                Ok(keyspaces) => {
                    println!("📋 Keyspaces found: {:?}", keyspaces);
                }
                Err(e) => {
                    println!("❌ Failed to list keyspaces: {}", e);
                }
            }
            
            // List tables in guruband
            match cassandra::list_tables(&session, "guruband").await {
                Ok(tables) => {
                    println!("📋 Tables in guruband: {:?}", tables);
                }
                Err(e) => {
                    println!("❌ Failed to list tables: {}", e);
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to connect: {}", e);
        }
    }
}