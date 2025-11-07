use crate::daemon_client;
use crate::shell::zsh;
use anyhow::{Context, Result};
use envhist_core::Config;
use envhist_daemon::EnvEvent;
use std::process::{Command, Stdio};

pub fn init(check: bool) -> Result<()> {
    if check {
        return check_installation();
    }

    println!("Initializing envhist...");

    // Ensure directories exist
    let storage = envhist_core::Storage::new()?;
    storage.ensure_directories()?;

    // Install shell hooks
    let zshrc_path = dirs::home_dir()
        .context("Failed to find home directory")?
        .join(".zshrc");

    zsh::install_hooks(&zshrc_path)
        .with_context(|| format!("Failed to install hooks to {:?}", zshrc_path))?;

    println!("✓ Installed shell hooks to ~/.zshrc");

    // Start daemon if not running
    if !is_daemon_running()? {
        start_daemon()?;
        println!("✓ Started daemon");
    } else {
        println!("✓ Daemon is already running");
    }

    println!("\nenvhist is ready! Restart your shell or run:");
    println!("  source ~/.zshrc");

    Ok(())
}

fn check_installation() -> Result<()> {
    let zshrc_path = dirs::home_dir()
        .context("Failed to find home directory")?
        .join(".zshrc");

    let zshrc_content = std::fs::read_to_string(&zshrc_path).unwrap_or_else(|_| String::new());

    if zshrc_content.contains("# envhist shell integration") {
        println!("✓ Shell hooks installed in ~/.zshrc");
    } else {
        println!("✗ Shell hooks not found in ~/.zshrc");
    }

    if is_daemon_running()? {
        println!("✓ Daemon is running");
    } else {
        println!("✗ Daemon is not running");
    }

    Ok(())
}

fn is_daemon_running() -> Result<bool> {
    let socket_path = Config::daemon_socket_path();
    Ok(socket_path.exists())
}

pub fn start_daemon() -> Result<()> {
    let exe_path = std::env::current_exe()?;

    Command::new(&exe_path)
        .arg("daemon")
        .arg("run")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| format!("Failed to start daemon using {:?}", exe_path))?;

    // Wait a bit for daemon to start
    std::thread::sleep(std::time::Duration::from_millis(500));

    Ok(())
}

pub fn run_daemon() -> Result<()> {
    let daemon = envhist_daemon::EnvHistDaemon::new()?;
    let socket_path = Config::daemon_socket_path();

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("Failed to build Tokio runtime")?;

    runtime
        .block_on(async { daemon.run(socket_path).await })
        .context("Daemon exited unexpectedly")?;

    Ok(())
}

pub fn stop_daemon() -> Result<()> {
    // Find daemon process and kill it
    let socket_path = Config::daemon_socket_path();
    if !socket_path.exists() {
        println!("Daemon is not running");
        return Ok(());
    }

    // Try to find the daemon process
    let output = Command::new("lsof")
        .arg("-t")
        .arg(socket_path.to_string_lossy().as_ref())
        .output()?;

    if output.stdout.is_empty() {
        println!("Could not find daemon process");
        return Ok(());
    }

    let pid_str = String::from_utf8(output.stdout)?.trim().to_string();

    if let Ok(pid) = pid_str.parse::<u32>() {
        Command::new("kill").arg(pid.to_string()).output()?;
        println!("✓ Stopped daemon (PID: {})", pid);
    }

    Ok(())
}

pub fn daemon_status() -> Result<()> {
    let socket_path = Config::daemon_socket_path();

    if socket_path.exists() {
        println!("✓ Daemon is running");
        println!("  Socket: {:?}", socket_path);

        // Try to get PID
        let output = Command::new("lsof")
            .arg("-t")
            .arg(socket_path.to_string_lossy().as_ref())
            .output();

        if let Ok(output) = output {
            if !output.stdout.is_empty() {
                let pid = String::from_utf8(output.stdout)?.trim().to_string();
                println!("  PID: {}", pid);
            }
        }
    } else {
        println!("✗ Daemon is not running");
    }

    Ok(())
}

pub fn send_set(pid: u32, key: String, value: String) -> Result<()> {
    let event = EnvEvent::Set { pid, key, value };
    let _ = daemon_client::send_event(event)?;
    Ok(())
}

pub fn send_unset(pid: u32, key: String) -> Result<()> {
    let event = EnvEvent::Unset { pid, key };
    let _ = daemon_client::send_event(event)?;
    Ok(())
}

pub fn send_capture(pid: u32) -> Result<()> {
    use envhist_core::Env;
    let env: Env = std::env::vars().collect();
    let event = EnvEvent::Capture { pid, env };
    let _ = daemon_client::send_event(event)?;
    Ok(())
}
