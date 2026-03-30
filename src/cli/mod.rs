use clap::{Parser, Subcommand};

pub mod manager;

#[derive(Parser)]
#[command(name = "surrealmcp")]
#[command(about = "SurrealDB MCP Server")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start the MCP server
    Start {
        /// The SurrealDB endpoint URL to connect to
        #[arg(short, long, env = "SURREALDB_URL")]
        endpoint: Option<String>,
        /// The SurrealDB namespace to use
        #[arg(long, env = "SURREALDB_NS")]
        ns: Option<String>,
        /// The SurrealDB database to use
        #[arg(long, env = "SURREALDB_DB")]
        db: Option<String>,
        /// The SurrealDB username to use
        #[arg(short, long, env = "SURREALDB_USER")]
        user: Option<String>,
        /// The SurrealDB password to use
        #[arg(short, long, env = "SURREALDB_PASS")]
        pass: Option<String>,
        /// The MCP server bind address (host:port)
        #[arg(long, env = "SURREAL_MCP_BIND_ADDRESS", group = "server")]
        bind_address: Option<String>,
        /// The MCP server Unix socket path
        #[arg(long, env = "SURREAL_MCP_SOCKET_PATH", group = "server")]
        socket_path: Option<String>,
        /// Rate limit requests per second (default: 100)
        #[arg(long, env = "SURREAL_MCP_RATE_LIMIT_RPS", default_value = "100")]
        rate_limit_rps: u32,
        /// Rate limit burst size (default: 200)
        #[arg(long, env = "SURREAL_MCP_RATE_LIMIT_BURST", default_value = "200")]
        rate_limit_burst: u32,
        /// Whether to require authentication for the MCP server
        #[arg(long, env = "SURREAL_MCP_AUTH_DISABLED", default_value = "false")]
        auth_disabled: bool,
        /// The URL address that the MCP server is accessible at
        #[arg(
            long,
            env = "SURREAL_MCP_SERVER_URL",
            default_value = "https://mcp.surrealdb.com"
        )]
        server_url: String,
        /// The SurrealDB Cloud authentication server URL
        #[arg(
            long,
            env = "SURREAL_MCP_AUTH_SERVER",
            default_value = "https://auth.surrealdb.com"
        )]
        auth_server: String,
        /// The expected audience for authentication tokens
        #[arg(
            long,
            env = "SURREAL_MCP_AUTH_AUDIENCE",
            default_value = "https://mcp.surrealdb.com/"
        )]
        auth_audience: String,
        /// SurrealDB Cloud access token (used instead of fetching tokens)
        #[arg(long, env = "SURREAL_MCP_CLOUD_ACCESS_TOKEN")]
        cloud_access_token: Option<String>,
        /// SurrealDB Cloud refresh token (used instead of fetching tokens)
        #[arg(long, env = "SURREAL_MCP_CLOUD_REFRESH_TOKEN")]
        cloud_refresh_token: Option<String>,
    },
    /// Stop a running MCP server
    Stop {
        /// The bind address of the server to stop (host:port)
        #[arg(long, short, default_value = "127.0.0.1:8090")]
        bind_address: String,
        /// Force stop even if server doesn't respond gracefully
        #[arg(long, short)]
        force: bool,
    },
    /// Check if MCP server is running
    Status {
        /// The bind address to check (host:port)
        #[arg(long, short, default_value = "127.0.0.1:8090")]
        bind_address: String,
    },
    /// Restart the MCP server
    Restart {
        /// The bind address of the server to restart (host:port)
        #[arg(long, short, default_value = "127.0.0.1:8090")]
        bind_address: String,
        /// Force stop even if server doesn't respond gracefully
        #[arg(long, short)]
        force: bool,
        /// The SurrealDB endpoint URL to connect to
        #[arg(short, long, env = "SURREALDB_URL")]
        endpoint: Option<String>,
        /// The SurrealDB namespace to use
        #[arg(long, env = "SURREALDB_NS")]
        ns: Option<String>,
        /// The SurrealDB database to use
        #[arg(long, env = "SURREALDB_DB")]
        db: Option<String>,
        /// The SurrealDB username to use
        #[arg(short, long, env = "SURREALDB_USER")]
        user: Option<String>,
        /// The SurrealDB password to use
        #[arg(short, long, env = "SURREALDB_PASS")]
        pass: Option<String>,
        /// The MCP server bind address (host:port)
        #[arg(long, env = "SURREAL_MCP_BIND_ADDRESS", group = "server")]
        new_bind_address: Option<String>,
        /// The MCP server Unix socket path
        #[arg(long, env = "SURREAL_MCP_SOCKET_PATH", group = "server")]
        socket_path: Option<String>,
        /// Rate limit requests per second (default: 100)
        #[arg(long, env = "SURREAL_MCP_RATE_LIMIT_RPS")]
        rate_limit_rps: Option<u32>,
        /// Rate limit burst size (default: 200)
        #[arg(long, env = "SURREAL_MCP_RATE_LIMIT_BURST")]
        rate_limit_burst: Option<u32>,
        /// Whether to require authentication for the MCP server
        #[arg(long, env = "SURREAL_MCP_AUTH_DISABLED", action = clap::ArgAction::Set)]
        auth_disabled: Option<bool>,
        /// The URL address that the MCP server is accessible at
        #[arg(long, env = "SURREAL_MCP_SERVER_URL")]
        server_url: Option<String>,
        /// The SurrealDB Cloud authentication server URL
        #[arg(long, env = "SURREAL_MCP_AUTH_SERVER")]
        auth_server: Option<String>,
        /// The expected audience for authentication tokens
        #[arg(long, env = "SURREAL_MCP_AUTH_AUDIENCE")]
        auth_audience: Option<String>,
        /// SurrealDB Cloud access token
        #[arg(long, env = "SURREAL_MCP_CLOUD_ACCESS_TOKEN")]
        cloud_access_token: Option<String>,
        /// SurrealDB Cloud refresh token
        #[arg(long, env = "SURREAL_MCP_CLOUD_REFRESH_TOKEN")]
        cloud_refresh_token: Option<String>,
    },
}
