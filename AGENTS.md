# CheenHub Agent Rules

## Project Shape

- Keep code simple first, then extend it through clear module boundaries.
- Prefer vertical feature modules over shared horizontal folders when adding product behavior.
- Do not add repository/service traits, generic abstraction layers, macros, or domain entities before they solve a real problem.
- Each file should have a current purpose: startup, config, telemetry, database, HTTP shell, contracts, migrations, UI feature, or styling.
- Use GUID/UUID values for persistent identifiers; expose them at API boundaries as strings only when the wire format requires it.

## Dioxus State

- Prefer local component state with Dioxus signals/events.
- Do not introduce global state, shared state modules, or context providers unless several independent feature boundaries need the same state.
- Keep component props explicit and small.
- Keep Dioxus components isolated: a file must not define more than one component.
- Prefer a component instance per rendered item over reusing a component instance across multiple items.
- When UI state belongs to a specific persistent entity, such as the selected server, room, text channel, voice room, or media session, render a keyed per-entity wrapper component and keep that entity-scoped state inside it instead of passing an optional active entity through long-lived siblings.
- Do not use direct `web_sys`, `js_sys`, JavaScript snippets, or browser APIs without explicit approval; prefer Dioxus-provided APIs such as Dioxus storage/events.

## Client Styling

- User-facing UI must feel welcoming and complete; do not show development-only technical details, TODO text, placeholder copy, or messages that explicitly frame a page as unfinished.
- Every UI area that waits for async data or an async action must include a loader/loading state.
- Every list that can be empty because of data, filters, sync errors, or first-run state must include a user-friendly empty state with a clear next action.
- Use Dioxus CLI Tailwind autodetection for the client.
- Keep Tailwind input files in `crates/client`; do not add root npm scripts, `package.json`, or a local `node_modules` styling pipeline.
- Do not try to start `dx serve` by default; assume the Dioxus dev server is usually already running in the background unless the user explicitly asks to start or restart it.

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
- Backend product features should use vertical layered modules when they contain real behavior: `transport` for HTTP adapters, `application` for use cases, `domain` for feature data/rules, `infrastructure` for database/external adapters, and `security` for auth/crypto primitives.
- Keep layer boundaries concrete: transport must not contain business rules or SQL, application must orchestrate behavior without HTTP response types, and infrastructure must not decide user-facing API errors.
- Do not introduce repository traits or service traits just to satisfy layering; use concrete modules/functions until multiple implementations are actually needed.
- Do not use raw SQL when SeaORM entities, SeaQuery, migration DSL, or another structured database API can express the operation clearly; reserve raw SQL for database-specific queries that the structured APIs cannot represent cleanly, and keep it isolated in infrastructure or migrations.
- In-memory infrastructure implementations are only for local testing and development; keep them maximally simple, deterministic, and free of production-style indexing, caching, cleanup jobs, or database emulation unless a test explicitly requires it.

## Configuration

- Local configuration is loaded from `.env`.
- Keep local database credentials in `.env`; do not commit local secrets or passwords.
- Do not add Docker Compose unless explicitly requested.
