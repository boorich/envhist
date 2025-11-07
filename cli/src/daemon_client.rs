use anyhow::Result;
use envhist_core::Config;
use envhist_daemon::EnvEvent;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::time::Duration;

pub fn send_event(event: EnvEvent) -> Result<()> {
    let socket_path = Config::daemon_socket_path();

    if !socket_path.exists() {
        // Daemon not running, silently fail
        return Ok(());
    }

    let mut stream = UnixStream::connect(&socket_path)
        .with_context(|| format!("Failed to connect to daemon socket {:?}", socket_path))?;

    stream.set_write_timeout(Some(Duration::from_millis(100)))?;
    stream.set_read_timeout(Some(Duration::from_millis(100)))?;

    let event_json = serde_json::to_string(&event)?;
    writeln!(stream, "{}", event_json)?;
    stream.flush()?;

    // Read response (ignore errors)
    let _ = stream.read_to_end(&mut Vec::new());

    Ok(())
}

use anyhow::Context;
