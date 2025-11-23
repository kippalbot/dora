# Repository Guidelines

## Project Structure & Module Organization
- Source Rust crates live in `libraries/` and `binaries/`; keep helpers near their callers to preserve module cohesion.
- Runnable dataflows belong in `examples/`, while reusable Python nodes live under `node-hub/`.
- Language bindings and SDKs sit in `apis/`; integration scenarios and end-to-end checks go in `tests/`.
- Documentation belongs in `docs/` or root-level Markdown files; avoid scattering design notes inside code directories.

## Build, Test, and Development Commands
- `cargo check --all --exclude dora-dav1d --exclude dora-rav1e`: fast structural validation of the Rust workspace.
- `cargo test --all --exclude dora-dav1d --exclude dora-rav1e --exclude dora-node-api-python --exclude dora-operator-api-python --exclude dora-ros2-bridge-python`: run the standard suite.
- `cargo fmt --all` and `uv run ruff check .`: enforce Rust and Python formatting/linting before committing.
- `cargo clippy --all`: surface Rust correctness and style issues early.
- `uv venv --seed -p 3.12 && uv pip install -e apis/python/node && uv run pytest`: prepare and validate the Python node API.
- `dora build && dora start --detach`: validate dataflows prior to publishing examples.

## Coding Style & Naming Conventions
- Rust uses rustfmt defaults; crates are kebab-case, modules/functions snake_case, types CamelCase.
- Python modules stay snake_case; rely on Ruff to enforce imports and spacing.
- Prefer `eyre`/`anyhow` for Rust error contexts and typed exceptions in Python; document public APIs with `///` comments or docstrings.

## Testing Guidelines
- Keep unit tests close to the code under test and name them after the behaviour under scrutiny.
- Place integration suites in `tests/` and combine `cargo test` with `dora start` when verifying dataflow interactions.
- Python tests reside in `tests/test_*.py`; use fixtures to gate GPU or network-dependent scenarios.
- Treat flakiness as a defect and add regression coverage before closing a fix.

## Commit & Pull Request Guidelines
- Write commits in the imperative mood (e.g., "Add telemetry exporter") and group related changes.
- Summaries should explain the problem, highlight operator/API impacts, and link issues when available.
- Include reproduction steps or `cargo test` output for runtime changes; wait for CI to pass before merging.

## Security & Environment Notes
- Keep API keys and secrets outside the repo; reference `test_env_api_key.sh` for expected environment variables.
- Review scripts in `docker/` and platform installers before running them.
- On macOS, prefer `uv` over global `pip` to keep Python dependencies reproducible.
