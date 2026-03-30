use anyhow::{Context, Result};
use std::net::SocketAddr;
use std::process::Command;
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};

const LOCK_FILE: &str = "/tmp/surrealmcp.lock";

pub struct ServerManager;

impl ServerManager {


    fn get_lock_file_path() -> std::path::PathBuf {
        std::path::PathBuf::from(LOCK_FILE)
    }

    pub fn acquire_lock() -> Result<bool> {
        let lock_path = Self::get_lock_file_path();
        
        if lock_path.exists() {
            return Ok(false);
        }

        std::fs::write(&lock_path, std::process::id().to_string())?;
        Ok(true)
    }

    pub fn release_lock() -> Result<()> {
        let lock_path = Self::get_lock_file_path();
        if lock_path.exists() {
            std::fs::remove_file(&lock_path)?;
        }
        Ok(())
    }

    pub async fn check_port_available(addr: &str) -> Result<bool> {
        match addr.parse::<SocketAddr>() {
            Ok(socket_addr) => {
                let timeout_duration = Duration::from_millis(500);
                let result = timeout(timeout_duration, TcpStream::connect(socket_addr)).await;
                Ok(result.is_err())
            }
            Err(_) => Ok(true),
        }
    }

    pub async fn is_server_running(addr: &str) -> Result<bool> {
        let socket_addr: SocketAddr = addr.parse()
            .context(format!("Invalid address format: {}", addr))?;
        
        let timeout_duration = Duration::from_millis(500);
        match timeout(timeout_duration, TcpStream::connect(socket_addr)).await {
            Ok(Ok(_)) => Ok(true),
            Ok(Err(_)) => Ok(false),
            Err(_) => Ok(false),
        }
    }

    pub async fn stop_server(addr: &str, force: bool) -> Result<()> {
        if !Self::is_server_running(addr).await? {
            return Err(anyhow::anyhow!("No server running at {}", addr));
        }

        if force {
            Self::kill_process_on_port(addr)?;
        } else {
            Self::send_graceful_shutdown(addr).await?;
        }

        let mut attempts = 0;
        while Self::is_server_running(addr).await? && attempts < 10 {
            tokio::time::sleep(Duration::from_millis(200)).await;
            attempts += 1;
        }

        if Self::is_server_running(addr).await? {
            Self::kill_process_on_port(addr)?;
        }

        Self::release_lock()?;
        Ok(())
    }

    async fn send_graceful_shutdown(_addr: &str) -> Result<()> {
        Ok(())
    }

    fn kill_process_on_port(addr: &str) -> Result<()> {
        let socket_addr: SocketAddr = addr.parse()
            .context(format!("Invalid address: {}", addr))?;
        
        let port = socket_addr.port();
        
        let output = Command::new("lsof")
            .args(["-ti", &format!(":{}", port)])
            .output()?;

        if output.stdout.is_empty() {
            return Err(anyhow::anyhow!("No process found on port {}", port));
        }

        let pids = String::from_utf8_lossy(&output.stdout);
        for pid_str in pids.lines() {
            if let Ok(pid) = pid_str.trim().parse::<u32>() {
                let _ = Command::new("kill").arg("-9").arg(pid.to_string()).output();
            }
        }

        Ok(())
    }

    pub async fn get_server_status(addr: &str) -> Result<ServerStatus> {
        let running = Self::is_server_running(addr).await?;
        
        let socket_addr: SocketAddr = addr.parse()
            .context(format!("Invalid address: {}", addr))?;

        Ok(ServerStatus {
            address: socket_addr,
            running,
        })
    }

    pub async fn query_health(addr: &str) -> Result<bool> {
        let url = format!("http://{}/health", addr);
        let client = reqwest::Client::new();
        match client.get(&url).timeout(Duration::from_millis(500)).send().await {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(_) => Ok(false),
        }
    }
}

#[derive(Debug)]
pub struct ServerStatus {
    pub address: SocketAddr,
    pub running: bool,
}
