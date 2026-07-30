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
│   ├── src/                   portal sources
│   ├── plugins/               Vite font-subsetting plugin
│   └── tests/                 Vitest suites, incl. the firmware contract test
├── scripts/
│   ├── bump-version.sh        determines next release version from git-cliff and bumps Cargo.toml
│   └── security-audit.sh      runs cargo audit + npm audit, drives the tracking issue
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

These are exactly the checks CI runs.

```bash
# Rust firmware (Xtensa)
cargo fmt --check
cargo clippy --locked -- -D warnings
cargo build --release --locked
cargo audit

# Rust library and its tests (host)
export HOST_TARGET="$(rustc -vV | sed -n 's/^host: //p')"
export RUSTFLAGS='' CARGO_UNSTABLE_BUILD_STD='std,alloc,core,panic_unwind,proc_macro'
cargo clippy --lib --tests --locked --target "$HOST_TARGET" -- -D warnings
cargo test --lib --locked --target "$HOST_TARGET"
unset RUSTFLAGS CARGO_UNSTABLE_BUILD_STD

# Portal (web UI)
cd portal
npm ci
npm run lint
npm run lint:css
npm run typecheck
npm test
npm run build
npm audit
cd ..

# Shell
shfmt --diff scripts/
shellcheck scripts/*.sh

# Workflows
actionlint

# Formatting, Markdown
npx --yes prettier@3.9.6 --check .
npx --yes markdownlint-cli2 "**/*.md"
```

`npm run fix` in `portal/` applies the autofixable half of the portal lint and
formatting rules.

## Commit style

This project uses [Conventional Commits](https://www.conventionalcommits.org/). Commit messages drive automatic changelog generation and version bumping.

Format: `type(scope): imperative summary` – lower case, no trailing period. The
scope is optional but preferred, and names the area rather than the file:
`portal`, `eye`, `sound`, `storage`, `web`, `release`, `deps`.

| Prefix      | When to use                           |
| ----------- | ------------------------------------- |
| `feat:`     | New feature or expression             |
| `fix:`      | Bug fix                               |
| `perf:`     | Faster or smaller, same behaviour     |
| `refactor:` | Code change without behavior change   |
| `test:`     | Adding or updating tests              |
| `docs:`     | Documentation only                    |
| `style:`    | Formatting, no logic change           |
| `build:`    | Build system, portal dev-dependencies |
| `ci:`       | CI/CD changes                         |
| `chore:`    | Maintenance, dependency updates       |

In the body, explain **why** – the diff already says what. For a fix, state the
failure mode it prevents, concretely. Wrap at 72-80 columns.

Breaking changes carry `!` after the type and a `BREAKING CHANGE:` footer.

## Pull requests

- Keep PRs focused on a single concern.
- Reference any related issue in the PR description.
- All CI checks must pass. `validate.yml` gates each job on a path filter, so a
  docs-only PR skips the firmware jobs and still reports green – a skipped job
  counts as passing, an unstarted workflow does not.

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
