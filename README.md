<div align="center">
  <h1>git-ezy</h1>
  <p>
    <strong>A modern, interactive Terminal UI for Git</strong>
  </p>
  <p>
    <a href="https://github.com/Pallavrai/git-ezy/blob/main/LICENSE">
      <img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="MIT License">
    </a>
    <img src="https://img.shields.io/badge/rust-1.85.0+-orange.svg" alt="Rust 1.85+">
    <img src="https://img.shields.io/badge/status-alpha-yellow.svg" alt="Alpha">
  </p>
  <br>
</div>

**git-ezy** replaces standard Git CLI memorization with an interactive, visual terminal dashboard. Navigate your repository, stage files, view diffs, and more — all from your keyboard.

Built with [ratatui](https://github.com/ratatui/ratatui), [crossterm](https://github.com/crossterm-rs/crossterm), and [git2](https://github.com/rust-lang/git2-rs).

## Features

- **Split-screen TUI** — file list on the left, diff/details on the right
- **Interactive staging** — toggle file staging with Spacebar
- **Color-coded diff viewer** — additions, deletions, and headers in distinct colors
- **Tabbed interface** — Status, Stage Files, Commit, Branches views
- **Vim-style navigation** — j/k or arrow keys to move around
- **Live status** — real-time Git status from libgit2

## Installation

### Prerequisites

- Rust 1.85+ (edition 2024)
- A Git repository (duh)

### From source

```bash
git clone https://github.com/Pallavrai/git-ezy.git
cd git-ezy
cargo build --release
./target/release/git-ezy
```

### Via Cargo (once published)

```bash
cargo install git-ezy
```

## Usage

Run `git-ezy` from inside any Git repository:

```bash
cd /path/to/your/repo
git-ezy
```

### Keybindings

| Key | Action |
|---|---|
| `↑` / `k` | Move selection up |
| `↓` / `j` | Move selection down |
| `Tab` | Next tab |
| `Shift+Tab` | Previous tab |
| `Space` | Toggle staging (Stage Files tab) |
| `Enter` | View diff of selected file (Status tab) |
| `q` / `Esc` | Quit |

### Tabs

| Tab | Description |
|---|---|
| **Status** | View modified files and their diffs |
| **Stage Files** | Stage/unstage files with Spacebar |
| **Commit** | Commit view (coming soon) |
| **Branches** | Branch management (coming soon) |

### Color coding

| Color | Meaning |
|---|---|
| **Cyan** | Active tab, selected item, branch name |
| **Green** | Staged files, diff additions |
| **Red** | Deleted files, diff deletions |
| **Yellow** | Modified/new files |
| `✓` | File is staged (Stage Files tab) |

## Project Status

This is an early-stage project. The following are planned:

- [ ] Commit message input and commit creation
- [ ] Branch checkout and creation
- [ ] Log/diff history view
- [ ] Mouse support
- [ ] Configurable keybindings

## Contributing

Contributions are welcome! Feel free to open issues or submit pull requests.

1. Fork the repository
2. Create a feature branch (`git checkout -b feat/my-feature`)
3. Make your changes
4. Run `cargo build` to verify compilation
5. Commit using conventional commits (`feat:`, `fix:`, etc.)
6. Push and open a PR

## License

MIT — see [LICENSE](LICENSE).

---

*Built with Rust, ratatui, and ❤️*
