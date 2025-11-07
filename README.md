# envhist

Git-style history for shell environment variables. `envhist` runs a lightweight daemon that records changes as you work so you can snapshot, diff, and restore your environment at any time.

## Getting Started

1. **Install the CLI**
   ```bash
   cargo install --path cli
   ```

2. **Initialize in your shell**
   ```bash
   envhist init
   ```
   This injects the zsh hook (Warp compatible) and starts the background daemon. Open a new tab or `source ~/.zshrc` to activate it.

3. **Work with snapshots**
   ```bash
   envhist snapshot            # auto-named snapshot of current env
   envhist list                # show snapshots for this session
   envhist status              # compare current env vs last snapshot
   envhist diff snap-a snap-b  # diff any two snapshots (defaults to current)
   envhist restore snap-a      # apply snapshot (prints exports for your shell)
   envhist log                 # timeline of tracked changes
   envhist show VAR_NAME       # history for a single variable
   ```

## How It Works

- A daemon listens on a Unix socket and writes per-session timelines under `~/.envhist/sessions/`.
- Shell hooks wrap `export`/`unset` and periodically `capture` full env state so diffs stay accurate.
- The CLI asks the daemon for the active session and stores session-specific snapshots alongside global ones.

## Development

```bash
cargo check     # type-check everything
cargo fmt       # format
cargo test      # (coming soon)
```

The repo is organised into a Rust workspace (`core`, `daemon`, `cli`) to keep shared logic isolated from transport and UX layers.
