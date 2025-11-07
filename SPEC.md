# `envhist` - Git for Environment Variables

## Project Vision

A lightweight, automatic version control system for shell environment variables that operates transparently in the background, tracking changes to your runtime environment the same way git tracks files.

## Core Design Principles

1. **Automatic** - Tracks env changes without explicit commands (unless you want them)
2. **Local-first** - All data stays on your machine, encrypted by default
3. **Shell-agnostic** - Works with bash, zsh, fish
4. **Git-like UX** - Familiar commands and workflows
5. **Zero overhead** - Fast enough to run on every shell operation
6. **Privacy-aware** - Intelligent filtering of secrets, opt-in cloud sync

## Technical Architecture

### Components

```
envhist/
├── cli/                    # User-facing CLI (Rust)
├── daemon/                 # Background watcher (Rust)
├── shell-hooks/            # Shell integration (bash/zsh/fish)
├── core/                   # Core library (Rust)
│   ├── tracker.rs         # Env change detection
│   ├── storage.rs         # Timeline & snapshot storage
│   ├── differ.rs          # Diff engine
│   ├── crypto.rs          # Encryption for secrets
│   └── shell_integration.rs
└── tests/
```

### Data Storage

```
~/.envhist/
├── config.toml            # User preferences
├── sessions/              # Per-shell-session data
│   └── <session-uuid>/
│       ├── timeline.jsonl # Append-only log of changes
│       ├── snapshots/     # Named environment states
│       │   ├── canton-dev.json
│       │   └── oauth-test.json
│       └── metadata.json  # Session info (started, shell type, etc.)
├── global/                # Cross-session data
│   ├── snapshots/         # Global snapshots
│   └── templates/         # Reusable env templates
└── .key                   # Encryption key (generated on init)
```

### Timeline Format (JSONL)

```jsonl
{"timestamp":"2025-11-07T10:23:45Z","action":"set","key":"CANTON_NODE_1","value":"0x742d35Cc6...","prev":null}
{"timestamp":"2025-11-07T10:24:12Z","action":"set","key":"CANTON_NODE_1","value":"0x8f3a29Bd8...","prev":"0x742d35Cc6..."}
{"timestamp":"2025-11-07T10:25:03Z","action":"unset","key":"OLD_API_KEY","prev":"sk-..."}
```

### Snapshot Format (JSON)

```json
{
  "name": "canton-dev",
  "created_at": "2025-11-07T10:30:00Z",
  "description": "Canton development network - node addresses after reset",
  "environment": {
    "CANTON_NODE_1": "0x8f3a29Bd8...",
    "CANTON_NODE_2": "0x1c7d84Ef2...",
    "CANTON_RPC_URL": "http://localhost:8545"
  },
  "tags": ["canton", "dev", "blockchain"],
  "session_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

## Core Features (MVP)

### 1. Shell Integration

**Automatic tracking via shell hooks:**

```bash
# In .bashrc/.zshrc after `envhist init`
eval "$(envhist shell-init)"
```

This injects:
- A wrapper around `export` that logs to daemon
- A `PROMPT_COMMAND` hook to capture env state before each prompt
- Cleanup on shell exit

**Rust daemon:**
- Listens on Unix socket for env change events
- Writes to timeline.jsonl
- Minimal CPU/memory footprint

### 2. CLI Commands

```bash
# Initialization
envhist init                           # Set up envhist in current shell
envhist init --global                  # Set up for all future shells

# Status & Inspection
envhist status                         # Show changed vars since last snapshot
envhist log                            # Show timeline of all changes
envhist log --since "1 hour ago"       # Filtered timeline
envhist log --grep CANTON              # Search timeline
envhist diff                           # Diff current vs last snapshot
envhist diff snapshot-1 snapshot-2     # Diff two snapshots
envhist show <var-name>                # Show history of a specific variable

# Snapshots
envhist snapshot <name>                # Save current env as named snapshot
envhist snapshot --auto                # Auto-generate name with timestamp
envhist list                           # List all snapshots
envhist restore <name>                 # Load a snapshot (sets vars in current shell)
envhist restore <name> --dry-run       # Preview what would be restored
envhist delete <name>                  # Delete a snapshot
envhist tag <name> <tag>               # Add tag to snapshot

# Watching
envhist watch                          # Live view of env changes
envhist watch CANTON_*                 # Watch specific pattern

# Export/Import
envhist export <name> > backup.env     # Export as .env file
envhist import backup.env              # Import and apply .env file
envhist export <name> --json           # Export as JSON

# Filtering & Privacy
envhist filter add "AWS_*"             # Never track AWS_* vars
envhist filter add ".*SECRET.*"        # Regex support
envhist filter list                    # Show filter rules
envhist encrypt <snapshot>             # Encrypt a snapshot
```

### 3. Auto-tracking Behavior

**What gets tracked:**
- All `export VAR=value` commands
- Changes detected via `PROMPT_COMMAND` hook
- Vars that existed and then disappeared (unset)

**What gets filtered (configurable):**
```toml
# ~/.envhist/config.toml
[filters]
# Default filters (security)
ignore_patterns = [
    ".*PASSWORD.*",
    ".*SECRET.*",
    ".*TOKEN.*",
    "AWS_.*",
    "SSH_.*"
]

# Track these despite matching ignore patterns
force_track = [
    "OAUTH_TEST_TOKEN",  # Explicitly want to track test tokens
]

# System vars to always ignore
ignore_system = [
    "PATH", "HOME", "USER", "SHELL", "PWD", 
    "OLDPWD", "TERM", "SHLVL", "_", "LS_COLORS"
]
```

### 4. Diff Engine

**Visual diff output:**
```bash
$ envhist diff canton-v1 canton-v2

--- canton-v1  (2025-11-07 10:30:00)
+++ canton-v2  (2025-11-07 11:45:00)

  CANTON_RPC_URL: http://localhost:8545
- CANTON_NODE_1: 0x742d35Cc6634Kb8f3e9B7a2Cd5...
+ CANTON_NODE_1: 0x8f3a29Bd8771Lc9g4f0C8b3De6...
- CANTON_NODE_2: 0x1c7d84Ef2998Md0h5g1D9c4Ef7...
+ CANTON_NODE_2: 0x5a8b92Cf3445Ne1i6h2E0d5Fg8...
+ CANTON_ADMIN_KEY: 0x9d4c03Ag4556Of2j7i3F1e6Gh9...

3 changed, 1 added, 0 removed
```

## Advanced Features (Post-MVP)

### 1. Session Management
```bash
envhist sessions                       # List all active sessions
envhist sessions --history             # Include closed sessions
envhist session show <session-id>      # Show session timeline
envhist session merge <sess-1> <sess-2> # Merge timelines
```

### 2. Templates
```bash
envhist template create dev-setup      # Save current env as template
envhist template apply dev-setup       # Apply template to new session
envhist template edit dev-setup        # Edit template vars
```

### 3. Git-like Branching (Maybe?)
```bash
envhist branch feature/new-network     # Create env branch
envhist checkout main                  # Switch env branches
envhist merge feature/new-network      # Merge env changes
```

### 4. Team Collaboration (Careful with secrets!)
```bash
envhist remote add origin git@...      # Add remote (encrypted)
envhist push                           # Push snapshots (secrets filtered)
envhist pull                           # Pull team snapshots
```

### 5. IDE Integration
- VSCode extension showing env history
- Cursor integration for AI-assisted env management
- Timeline visualization UI

## Technical Implementation Details

### Process Monitoring (Linux/macOS)

**For tracking other terminals:**
```rust
// Read env from running process
fn read_process_env(pid: u32) -> Result<HashMap<String, String>> {
    let environ_path = format!("/proc/{}/environ", pid);
    let content = fs::read(environ_path)?;
    
    // Parse null-terminated strings
    content
        .split(|&b| b == 0)
        .filter_map(|s| {
            let s = String::from_utf8_lossy(s);
            let mut parts = s.splitn(2, '=');
            Some((parts.next()?.to_string(), parts.next()?.to_string()))
        })
        .collect()
}
```

### Shell Hook (Bash Example)

```bash
# Generated by `envhist shell-init`
_envhist_export() {
    # Send to daemon via Unix socket
    echo "$@" | nc -U ~/.envhist/daemon.sock
    # Actually do the export
    builtin export "$@"
}

alias export='_envhist_export'

_envhist_prompt() {
    # Capture env state before prompt
    envhist capture --pid $$ 2>/dev/null
}

PROMPT_COMMAND="_envhist_prompt${PROMPT_COMMAND:+;$PROMPT_COMMAND}"

# Cleanup on exit
trap 'envhist session-close --pid $$' EXIT
```

### Daemon Architecture

```rust
// Long-running background process
struct EnvHistDaemon {
    socket: UnixListener,
    sessions: HashMap<Uuid, Session>,
    storage: Storage,
}

impl EnvHistDaemon {
    async fn handle_event(&mut self, event: EnvEvent) {
        match event {
            EnvEvent::Set { pid, key, value } => {
                let session = self.get_or_create_session(pid);
                session.record_change(key, value);
                self.storage.append_timeline(session.id, &change);
            }
            EnvEvent::Capture { pid, env } => {
                let session = self.get_or_create_session(pid);
                session.update_full_state(env);
            }
            // ...
        }
    }
}
```

### Diff Algorithm

```rust
#[derive(Debug)]
enum EnvDiff {
    Added { key: String, value: String },
    Removed { key: String, old_value: String },
    Changed { key: String, old_value: String, new_value: String },
    Unchanged { key: String, value: String },
}

fn diff_envs(old: &Env, new: &Env) -> Vec<EnvDiff> {
    let mut diffs = Vec::new();
    
    // Find added and changed
    for (key, new_val) in new.iter() {
        match old.get(key) {
            None => diffs.push(EnvDiff::Added { 
                key: key.clone(), 
                value: new_val.clone() 
            }),
            Some(old_val) if old_val != new_val => {
                diffs.push(EnvDiff::Changed {
                    key: key.clone(),
                    old_value: old_val.clone(),
                    new_value: new_val.clone(),
                })
            }
            Some(_) => {} // Unchanged
        }
    }
    
    // Find removed
    for (key, old_val) in old.iter() {
        if !new.contains_key(key) {
            diffs.push(EnvDiff::Removed {
                key: key.clone(),
                old_value: old_val.clone(),
            });
        }
    }
    
    diffs
}
```

### Encryption

```rust
use aes_gcm::{Aes256Gcm, Key, Nonce};
use argon2::Argon2;

// Generate key on first init
fn init_encryption() -> Result<Key<Aes256Gcm>> {
    let mut key = [0u8; 32];
    OsRng.fill_bytes(&mut key);
    
    // Store encrypted with user password
    let key = Key::<Aes256Gcm>::from_slice(&key);
    Ok(*key)
}

// Encrypt sensitive values
fn encrypt_value(key: &Key<Aes256Gcm>, value: &str) -> Result<String> {
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(b"unique nonce");
    
    let ciphertext = cipher.encrypt(nonce, value.as_bytes())?;
    Ok(base64::encode(ciphertext))
}
```

## Configuration

```toml
# ~/.envhist/config.toml

[core]
auto_snapshot = true               # Auto-snapshot on shell exit
auto_snapshot_interval = 3600      # Seconds between auto-snapshots
max_timeline_size = 10000          # Max entries before rotation
daemon_enabled = true              # Run background daemon

[filters]
ignore_patterns = [".*PASSWORD.*", ".*SECRET.*"]
force_track = []
ignore_system = ["PATH", "HOME", "USER"]

[display]
diff_context = 3                   # Lines of context in diffs
color = true                       # Colorize output
timezone = "local"                 # Timestamp display

[encryption]
enabled = true                     # Encrypt snapshots
algorithm = "aes-256-gcm"
auto_encrypt_patterns = [".*KEY.*", ".*TOKEN.*"]

[performance]
daemon_poll_interval = 1000        # Milliseconds
max_memory_mb = 50                 # Max daemon memory
```

## Installation & Distribution

```bash
# From source
cargo install envhist

# Homebrew (future)
brew install envhist

# Post-install setup
envhist setup                      # Interactive setup wizard
envhist setup --auto               # Automatic setup with defaults
```

## Testing Strategy

### Unit Tests
- Timeline parsing/writing
- Diff algorithm
- Encryption/decryption
- Filter matching

### Integration Tests
- Shell hook injection
- Daemon communication
- Snapshot save/restore
- Multi-session handling

### E2E Tests
```bash
# Test in real shells
tests/
├── bash_test.sh
├── zsh_test.sh
└── fish_test.sh
```

## Security Considerations

1. **Secret detection** - Pattern matching for common secret formats
2. **Encryption at rest** - All snapshots encrypted by default
3. **Secure IPC** - Unix sockets with proper permissions
4. **No network by default** - Purely local unless explicitly configured
5. **Audit log** - Track who accessed what snapshots (multi-user systems)

## Performance Goals

- **Daemon memory**: < 50MB
- **Hook overhead**: < 5ms per export
- **Snapshot restore**: < 100ms for 100 vars
- **Timeline query**: < 50ms for 10k entries
- **Diff calculation**: < 10ms for 1k vars

## Success Metrics

- **Adoption**: Used daily by developers with complex env setups
- **Reliability**: Zero data loss in timeline
- **Performance**: Unnoticeable overhead in shell
- **UX**: "Just works" without configuration

## Future Vision

- **Language SDKs** - Python/Node/Ruby libraries to interact with envhist
- **Cloud sync** - Encrypted backup to S3/GitHub
- **Team features** - Shared env templates (secrets filtered)
- **AI integration** - "Restore my env from when the tests were passing"
- **Time travel** - "What was CANTON_NODE_1 at 3pm yesterday?"

---

## Open Questions for Discussion

1. **Daemon vs. Hook-only?** - Should we require daemon or work purely with shell hooks?
2. **Secret handling** - Auto-redact, encrypt, or trust user filters?
3. **Cross-shell sessions** - How to handle tmux/screen where env changes in one pane affect others?
4. **Windows support** - Worth the complexity or Linux/macOS only for MVP?

---

**Ready to build this in Cursor! 🦀**
