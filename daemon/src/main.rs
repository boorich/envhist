use anyhow::Result;
use envhist_core::Config;
use envhist_daemon::server::EnvHistDaemon;

#[tokio::main]
async fn main() -> Result<()> {
    let daemon = EnvHistDaemon::new()?;
    let socket_path = Config::daemon_socket_path();

    daemon.run(socket_path).await?;

    Ok(())
}
