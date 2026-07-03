# Contributing to Crown of the Lamb

Thank you for considering a contribution. This document covers everything you need to get started.

## Overview

`no_std` Rust firmware for an ESP32-powered animatronic Cult of the Lamb crown prop. It drives a GC9A01 round display with an animated eye, detects sound direction from two microphones via FFT to react with facial expressions, and hosts its own WiFi access point with a captive-portal web UI for live configuration.

## Project structure

```text
.
├── src/                       firmware library (no_std)
│   ├── bin/
│   │   ├── main.rs            entry point + main animation loop
│   │   ├── reactor/           microphone sampling -> eye reactions
│   │   └── web/               captive portal HTTP handlers
│   ├── eye.rs                 eye state machine and expressions
│   ├── sound.rs               FFT direction analysis
│   ├── display.rs             GC9A01 rendering
│   └── config.rs / storage.rs runtime config + flash persistence
├── portal/                    TypeScript + Vite captive-portal web UI
├── scripts/
│   └── bump-version.sh        determines next release version from git-cliff and bumps Cargo.toml
├── partitions.csv             flash partition table
└── build.rs                   bundles the portal into the firmware
```

## Development setup

This project targets the Xtensa `esp` toolchain, so you need the ESP-RS tooling:

```bash
git clone https://github.com/wielorzeczownik/crown-of-the-lamb.git
cd crown-of-the-lamb

# One-time: install the Xtensa Rust toolchain and flashing tool
cargo install espup espflash
espup install --targets esp32
source "$HOME/export-esp.sh"   # add to your shell profile

# Flash + monitor a connected board
cargo run --release
```

The portal (web UI) is a separate npm package under `portal/`:

```bash
cd portal
npm ci
npm run dev          # develop the UI in a browser
npm run build        # build the bundle that build.rs embeds into the firmware
```

## Running checks locally

```bash
# Rust firmware
cargo fmt --check
cargo clippy -- -D warnings
cargo build --release

# Portal (web UI)
cd portal
npm run format:check
npm run lint
npm run lint:css
npm run typecheck

# Shell
shfmt --diff scripts/

# Markdown
markdownlint-cli2 "**/*.md"
```

## Commit style

This project uses [Conventional Commits](https://www.conventionalcommits.org/). Commit messages drive automatic changelog generation and version bumping.

Common prefixes:

| Prefix      | When to use                         |
| ----------- | ----------------------------------- |
| `feat:`     | New feature or expression           |
| `fix:`      | Bug fix                             |
| `test:`     | Adding or updating tests            |
| `chore:`    | Maintenance, dependency updates     |
| `refactor:` | Code change without behavior change |
| `docs:`     | Documentation only                  |
| `style:`    | Formatting, no logic change         |
| `ci:`       | CI/CD changes                       |

Breaking changes must include `BREAKING CHANGE:` in the commit footer.

## Pull requests

- Keep PRs focused on a single concern.
- Reference any related issue in the PR description.
- All CI checks must pass: rustfmt, clippy, firmware release build, portal lint/format/typecheck/build, shell and Markdown linting, and the vulnerability scan.
- Releases are automated: merging Conventional Commits to `main` bumps the version and publishes a tagged release with firmware `.bin` and generated notes.

## Reporting bugs

Open an [issue](https://github.com/wielorzeczownik/crown-of-the-lamb/issues) and include:

- What you did
- What you expected
- What actually happened
- Relevant serial logs (`defmt`) or error messages
- Your hardware (board, display, microphones) and how you flashed the firmware

> For security issues, please read [SECURITY.md](SECURITY.md) before opening a public issue.

## License

By contributing you agree that your changes will be licensed under the [MIT License](LICENSE).
