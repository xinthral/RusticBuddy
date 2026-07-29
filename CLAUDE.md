# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Purpose

A personal CLI fitness tracker. It stores two categories of data:
- **Personal metrics** — weight, heart rate, and other body data points over time
- **Workout tracking** — current workout plan, exercises, sets/reps/weights

## Commands

```bash
cargo build          # compile
cargo run            # build and run
cargo test           # run all tests
cargo test <name>    # run a single test by name
cargo clippy         # lint
```

## Dependency constraints

- **rusqlite must stay at `0.32.1`** — versions 0.40+ use the nightly-only `cfg_select!` macro and will not compile on stable Rust.
- The `bundled` feature is required on Windows (there is no system `sqlite3.lib`). Do not remove it from `Cargo.toml`.

## Architecture

This is an early-stage Rust CLI application. The module layout reflects an intended separation between data access and user interaction:

- `src/main.rs` — entry point; currently just opens a database connection via `load_database()`
- `src/database.rs` — owns the `Database` struct and the rusqlite `Connection`; `Database::connect()` returns `Result<Connection>` using an in-memory SQLite database
- `src/user.rs` — intended to hold user/profile domain types (`Profile`)
- `src/interface.rs` — intended to hold the API layer between the UI and the database (`DatabaseApi`, `UserApi`)

The planned data flow is: `main` → `interface` (API layer) → `database` (rusqlite). The `interface` and `user` modules are currently stubs.