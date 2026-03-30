use crate::server::ServerConfig;
use anyhow::Result;
use clap::Parser;
use cli::manager::ServerManager;

mod cli;
mod cloud;
mod db;
mod engine;
mod logs;
mod prompts;
mod resources;
mod server;
mod tools;
mod utils;

#[tokio::main]
async fn main() -> Result<()> {
    if rustls::crypto::ring::default_provider()
        .install_default()
        .is_err()
    {
        tracing::error!("Failed to install default crypto provider");
    }

    let cli = cli::Cli::parse();

    match cli.command {
        cli::Commands::Start {
            endpoint,
            ns,
            db,
            user,
            pass,
            server_url,
            bind_address,
            socket_path,
            auth_disabled,
            rate_limit_rps,
            rate_limit_burst,
            auth_server,
            auth_audience,
            cloud_access_token,
            cloud_refresh_token,
        } => {
            // Filter out empty strings from env vars (treat as None)
            let addr = bind_address.as_ref().filter(|s| !s.is_empty());
            let addr_str = addr.as_deref();
            
            if let Some(address) = addr_str {
                if ServerManager::check_port_available(address).await? {
                    return Err(anyhow::anyhow!(
                        "Address {} is already in use. Use 'surrealmcp stop' first or check if another server is running.",
                        address
                    ));
                }
            }

            if !ServerManager::acquire_lock()? {
                return Err(anyhow::anyhow!(
                    "Another instance is already running. Use 'surrealmcp stop' first."
                ));
            }

            // Filter empty strings from optional args
            let final_bind_address = bind_address.filter(|s| !s.is_empty());
            let final_socket_path = socket_path.filter(|s| !s.is_empty());
            let final_endpoint = endpoint.filter(|s| !s.is_empty());

            let config = ServerConfig {
                endpoint: final_endpoint,
                ns,
                db,
                user,
                pass,
                server_url,
                bind_address: final_bind_address,
                socket_path: final_socket_path,
                auth_disabled,
                rate_limit_rps,
                rate_limit_burst,
                auth_server,
                auth_audience,
                cloud_access_token,
                cloud_refresh_token,
            };

            let result = server::start_server(config).await;
            
            let _ = ServerManager::release_lock();
            
            result
        }
        cli::Commands::Stop { bind_address, force } => {
            println!("Stopping server at {}...", bind_address);
            ServerManager::stop_server(&bind_address, force).await?;
            println!("Server stopped successfully.");
            Ok(())
        }
        cli::Commands::Status { bind_address } => {
            let status = ServerManager::get_server_status(&bind_address).await?;
            if status.running {
                let healthy = ServerManager::query_health(&status.address.to_string()).await.unwrap_or(false);
                println!("Server is running at {} (health: {})", status.address, if healthy { "ok" } else { "unhealthy" });
            } else {
                println!("Server is not running at {}", bind_address);
            }
            Ok(())
        }
        cli::Commands::Restart {
            bind_address,
            force,
            endpoint,
            ns,
            db,
            user,
            pass,
            new_bind_address,
            socket_path,
            rate_limit_rps,
            rate_limit_burst,
            auth_disabled,
            server_url,
            auth_server,
            auth_audience,
            cloud_access_token,
            cloud_refresh_token,
        } => {
            // Stop the server if it's running
            if ServerManager::is_server_running(&bind_address).await? {
                println!("Stopping server at {}...", bind_address);
                ServerManager::stop_server(&bind_address, force).await?;
                println!("Server stopped.");
            }
            
            // Wait for port to be released
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;

            // Determine final bind address - use current if no new one specified
            let final_bind_address = match &new_bind_address {
                Some(addr) if !addr.is_empty() => addr.clone(),
                _ => bind_address.clone(),
            };

            // Filter empty strings from optional args
            let final_socket_path = socket_path.filter(|s| !s.is_empty());
            let final_endpoint = endpoint.filter(|s| !s.is_empty());

            // Build config using same defaults as start command
            let config = ServerConfig {
                endpoint: final_endpoint,
                ns,
                db,
                user,
                pass,
                server_url: server_url.unwrap_or_else(|| "https://mcp.surrealdb.com".to_string()),
                bind_address: Some(final_bind_address.clone()),
                socket_path: final_socket_path,
                auth_disabled: auth_disabled.unwrap_or(false),
                rate_limit_rps: rate_limit_rps.unwrap_or(100),
                rate_limit_burst: rate_limit_burst.unwrap_or(200),
                auth_server: auth_server.unwrap_or_else(|| "https://auth.surrealdb.com".to_string()),
                auth_audience: auth_audience.unwrap_or_else(|| "https://mcp.surrealdb.com/".to_string()),
                cloud_access_token,
                cloud_refresh_token,
            };

            // Skip port check - we just stopped the server so it should be available
            // Let start_server handle any actual bind errors
            if !ServerManager::acquire_lock()? {
                return Err(anyhow::anyhow!(
                    "Another instance is already running. Use 'surrealmcp stop' first."
                ));
            }

            println!("Starting server at {}...", final_bind_address);
            let result = server::start_server(config).await;
            
            let _ = ServerManager::release_lock();
            
            result
        }
    }
}
