use anyhow::Result;
use clap::{Parser, Subcommand};

mod commands;
mod daemon_client;
mod shell;

#[derive(Parser)]
#[command(name = "envhist")]
#[command(about = "Git for environment variables", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize envhist in your shell
    Init {
        /// Check installation status
        #[arg(long)]
        check: bool,
    },
    /// Save current environment as a snapshot
    Snapshot {
        /// Snapshot name (auto-generated if not provided)
        name: Option<String>,
        /// Description
        #[arg(short, long)]
        description: Option<String>,
    },
    /// List all snapshots
    List,
    /// Restore a snapshot
    Restore {
        /// Snapshot name
        name: String,
        /// Preview changes without applying
        #[arg(long)]
        dry_run: bool,
    },
    /// Delete a snapshot
    Delete {
        /// Snapshot name
        name: String,
    },
    /// Show changes since last snapshot
    Status,
    /// Show timeline of environment changes
    Log {
        /// Filter by time (e.g., "1 hour ago")
        #[arg(long)]
        since: Option<String>,
        /// Filter by variable name pattern
        #[arg(long)]
        grep: Option<String>,
    },
    /// Show history of a specific variable
    Show {
        /// Variable name
        name: String,
    },
    /// Show differences between environments
    Diff {
        /// First snapshot (or current if not provided)
        snapshot1: Option<String>,
        /// Second snapshot (or current if not provided)
        snapshot2: Option<String>,
    },
    /// Daemon management
    Daemon {
        #[command(subcommand)]
        action: DaemonCommand,
    },
    /// Send set event to daemon (internal use)
    SendSet {
        pid: u32,
        key: String,
        value: String,
    },
    /// Send unset event to daemon (internal use)
    SendUnset { pid: u32, key: String },
    /// Send capture event to daemon (internal use)
    SendCapture { pid: u32 },
}

#[derive(Subcommand)]
enum DaemonCommand {
    /// Start the daemon
    Start,
    /// Stop the daemon
    Stop,
    /// Check daemon status
    Status,
    /// Run the daemon (internal use)
    #[command(hide = true)]
    Run,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { check } => commands::init::init(check),
        Commands::Snapshot { name, description } => commands::snapshot::snapshot(name, description),
        Commands::List => commands::snapshot::list(),
        Commands::Restore { name, dry_run } => commands::snapshot::restore(name, dry_run),
        Commands::Delete { name } => commands::snapshot::delete(name),
        Commands::Status => commands::status::status(),
        Commands::Log { since, grep } => commands::log::log(since, grep),
        Commands::Show { name } => commands::log::show(name),
        Commands::Diff {
            snapshot1,
            snapshot2,
        } => commands::diff::diff(snapshot1, snapshot2),
        Commands::Daemon { action } => match action {
            DaemonCommand::Start => commands::init::start_daemon(),
            DaemonCommand::Stop => commands::init::stop_daemon(),
            DaemonCommand::Status => commands::init::daemon_status(),
            DaemonCommand::Run => commands::init::run_daemon(),
        },
        Commands::SendSet { pid, key, value } => commands::init::send_set(pid, key, value),
        Commands::SendUnset { pid, key } => commands::init::send_unset(pid, key),
        Commands::SendCapture { pid } => commands::init::send_capture(pid),
    }
}
