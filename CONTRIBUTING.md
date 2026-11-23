# Contributing

Every contribution helps, including a typo fix, a bug report, a new expression, or a question about how something works. For anything bigger than a small fix, opening an issue first is a good idea, so we can agree on the approach before you spend time on it.

## Setup

The firmware targets the Xtensa `esp` toolchain, so you need the ESP-RS tooling:

```bash
git clone https://github.com/wielorzeczownik/crown-of-the-lamb.git
cd crown-of-the-lamb

# once: the Xtensa Rust toolchain and the flashing tool
cargo install espup espflash
espup install --targets esp32
source "$HOME/export-esp.sh"   # worth adding to your shell profile

# flash and monitor a connected board
cargo run --release
```

The captive-portal web UI is a separate npm package in `portal/`:

```bash
cd portal
npm ci
npm run dev     # work on the UI in a browser
npm run build   # the bundle that build.rs embeds into the firmware
```

You can work on the portal without any hardware.

## Checks

CI runs these on every pull request. You don't have to run them all locally, since CI will point out anything that's off:

```bash
# firmware (Xtensa)
cargo fmt --check
cargo clippy --locked -- -D warnings
cargo build --release --locked

# library and its tests, on the host
export HOST_TARGET="$(rustc -vV | sed -n 's/^host: //p')"
export RUSTFLAGS='' CARGO_UNSTABLE_BUILD_STD='std,alloc,core,panic_unwind,proc_macro'
cargo clippy --lib --tests --locked --target "$HOST_TARGET" -- -D warnings
cargo test --lib --locked --target "$HOST_TARGET"
unset RUSTFLAGS CARGO_UNSTABLE_BUILD_STD

# portal
cd portal
npm ci
npm run lint
npm run lint:css
npm run typecheck
npm test
npm run build
cd ..

shfmt --diff scripts/
shellcheck scripts/*.sh
actionlint
npx prettier --check .
npx --yes markdownlint-cli2 "**/*.md"
```

`npm run fix` in `portal/` fixes what the portal linters and prettier can fix on their own.

CI only runs the jobs whose files changed, so a docs-only PR skips the firmware jobs and still shows green.

## Tests

Rust tests sit next to the code they cover and run on the host, not the board (the commands above). Portal tests are vitest suites in `portal/tests/`, including `firmware-contract.test.ts`, which reads the firmware source and checks that the portal still agrees with it.

## Commits and pull requests

Commits follow [Conventional Commits](https://www.conventionalcommits.org/) (`feat(eye): ...`, `fix(portal): ...`), because the changelog and version bumps are generated from them. The scope names the area, e.g. `portal`, `eye`, `sound`, `storage`, `web`. Don't worry if you get it wrong, it can be fixed before merging.

Smaller PRs that do one thing are easier to review. Tests for behavior changes are very welcome. If you're not sure how to test something on the device, open the PR anyway and we'll work it out together.

Bugs and ideas go to [issues](https://github.com/wielorzeczownik/crown-of-the-lamb/issues). Security problems go through [SECURITY.md](SECURITY.md).

Contributions are licensed under the [MIT License](LICENSE).
