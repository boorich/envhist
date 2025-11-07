use anyhow::{Context, Result};
use envhist_core::{session::Session, Config};
use envhist_daemon::{EnvEvent, EnvResponse};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::time::Duration;

pub fn send_event(event: EnvEvent) -> Result<Option<EnvResponse>> {
    let socket_path = Config::daemon_socket_path();

    if !socket_path.exists() {
        // Daemon not running, silently fail
        return Ok(None);
    }

    let mut stream = UnixStream::connect(&socket_path)
        .with_context(|| format!("Failed to connect to daemon socket {:?}", socket_path))?;

    stream.set_write_timeout(Some(Duration::from_millis(100)))?;
    stream.set_read_timeout(Some(Duration::from_millis(100)))?;

    let event_json = serde_json::to_string(&event)?;
    writeln!(stream, "{}", event_json)?;
    stream.flush()?;

    let mut reader = BufReader::new(&mut stream);
    let mut response_line = String::new();
    reader
        .read_line(&mut response_line)
        .context("Failed to read daemon response")?;

    if response_line.trim().is_empty() {
        return Ok(Some(EnvResponse::Ok));
    }

    let response: EnvResponse =
        serde_json::from_str(response_line.trim()).context("Failed to parse daemon response")?;

    Ok(Some(response))
}

pub fn get_session(pid: u32) -> Result<Option<Session>> {
    match send_event(EnvEvent::GetSession { pid })? {
        Some(EnvResponse::Session { session }) => Ok(Some(session)),
        Some(EnvResponse::Error { message }) => {
            anyhow::bail!("Daemon error fetching session: {}", message)
        }
        _ => Ok(None),
    }
}
