# CheenHub Agent Rules

## Project Shape

- Keep code simple first, then extend it through clear module boundaries.
- Prefer vertical feature modules over shared horizontal folders when adding product behavior.
- Do not add repository/service traits, generic abstraction layers, macros, or domain entities before they solve a real problem.
- Each file should have a current purpose: startup, config, telemetry, database, HTTP shell, contracts, migrations, UI feature, or styling.

## Dioxus State

- Prefer local component state with Dioxus signals/events.
- Do not introduce global state, shared state modules, or context providers unless several independent feature boundaries need the same state.
- Keep component props explicit and small.

## Client Styling

- Use Dioxus CLI Tailwind autodetection for the client.
- Keep Tailwind input files in `crates/client`; do not add root npm scripts, `package.json`, or a local `node_modules` styling pipeline.

## Public API Documentation

- Every crate must include crate-level documentation.
- Every public module, type, function, trait, enum, constant, and field must have `///` documentation when it is introduced.
- Crates use `#![warn(missing_docs)]`; warnings are acceptable during early development, but new public API should not add missing-doc warnings.
- Run `cargo clippy --workspace --all-targets` before handing off code.

## Backend

- REST is the default client-server control plane.
- WebTransport is reserved for voice/media transport.
- WebCodecs is reserved for browser-side audio/video processing.
- Do not implement voice rooms, authentication, WebTransport, or WebCodecs behavior until explicitly requested.

## Configuration

- Local configuration is loaded from `.env`.
- Keep local database credentials in `.env`; do not commit local secrets or passwords.
- Do not add Docker Compose unless explicitly requested.
