# CheenHub Agent Rules

## Project Shape

- Keep code simple first, then extend it through clear module boundaries.
- When several implementation paths are available and there is no obvious best choice, state the tradeoff and get explicit user approval before implementing.
- Prefer vertical feature modules over shared horizontal folders when adding product behavior.
- Treat feature and layer boundaries as hard design constraints. Do not move state, contracts, or behavior across those boundaries for convenience unless the user explicitly approves the boundary violation after the tradeoff is stated.
- Do not add repository/service traits, generic abstraction layers, macros, or domain entities before they solve a real problem.
- Each file should have a current purpose: startup, config, telemetry, database, HTTP shell, contracts, migrations, UI feature, or styling.
- Use GUID/UUID values for persistent identifiers; expose them at API boundaries as strings only when the wire format requires it.

## Dioxus State

- Prefer local component state with Dioxus signals/events.
- Prefer Dioxus-provided primitives over custom lifecycle state. For async data loading, use `use_resource` before adding manual `use_effect`/`spawn` guards such as `loaded_*` flags.
- Do not introduce global state, shared state modules, or context providers unless several independent feature boundaries need the same state.
- Keep component props explicit and small.
- Keep Dioxus components isolated: a file must define exactly one `#[component]`. Helper functions are allowed, but additional components must live in separate files.
- Prefer a component instance per rendered item over reusing a component instance across multiple items.
- When UI state belongs to a specific persistent entity, such as the selected server, room, text channel, voice room, or media session, render a keyed per-entity wrapper component and keep that entity-scoped state inside it instead of passing an optional active entity through long-lived siblings.
- Avoid prop drilling multiple unrelated callbacks through UI-only components. When a child component represents a menu, toolbar, or command surface with several actions, prefer a small feature-local action enum and a single `EventHandler<Action>` prop.
- Keep action enums local to the nearest feature or component boundary that owns the resulting state changes. Do not promote them to shared modules unless multiple feature boundaries use the same action contract.
- UI components should receive the data they render and emit user intent or completed local command outcomes; parent scopes should decide how that intent changes state, opens modals, or switches views.
- Use Dioxus context/providers only when the same state or commands are needed by several independent feature boundaries. Do not introduce context only to avoid passing one or two props within a single local component tree.
- Do not use direct `web_sys`, `js_sys`, JavaScript snippets, or browser APIs without explicit approval; prefer Dioxus-provided APIs such as Dioxus storage/events.

## Client Styling

- User-facing UI must feel welcoming and complete; do not show development-only technical details, TODO text, placeholder copy, or messages that explicitly frame a page as unfinished.
- Every UI area that waits for async data or an async action must include a loader/loading state.
- Every list that can be empty because of data, filters, sync errors, or first-run state must include a user-friendly empty state with a clear next action.
- Use Dioxus CLI Tailwind autodetection for the client.
- Keep Tailwind input files in `crates/client`; do not add root npm scripts, `package.json`, or a local `node_modules` styling pipeline.
- Do not try to start `dx serve` by default; assume the Dioxus dev server is usually already running in the background unless the user explicitly asks to start or restart it.

## Client Realtime

- Keep `crates/client/src/features/realtime` generic: connection setup, stream management, framing, generic request/send APIs, and generic inbound event subscription.
- Do not add feature-specific methods such as `send_text_message` or `load_room_history` to `RealtimeHandle`; feature modules should call generic `request`, `send_reliable`, or `send_unreliable` themselves.
- Feature-specific realtime request helpers and event decoding/filtering belong in the owning client feature, not in the generic realtime handle.

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

## Backend Realtime

- Keep `crates/backend/src/realtime` transport-focused: session lifecycle, stream framing, authentication stream handling, module routing, shared stream registries/fanout, TLS, and protocol helpers only.
- Do not put product feature behavior in backend `realtime/*` modules. Feature-specific realtime adapters belong under the owning feature, such as `features/text_chat/realtime.rs`.
- A realtime feature module should expose a message handler for envelopes addressed to that module. Do not add feature-specific bind/unbind lifecycle hooks unless the user explicitly approves the extra lifecycle contract.
- Shared realtime fanout and stream registries belong in `realtime`, not in a product feature, when the mechanism can serve multiple feature modules.
- Server-scoped broadcast APIs must require an explicit server identifier and must not broadcast across all servers by default.
- Room-level or resource-level visibility checks should remain feature policy layered on top of server-scoped realtime recipient filtering.

## Logging

- New backend and frontend behavior must include useful logs at important lifecycle and failure points, especially connection/session lifecycle, authentication decisions, rejected requests, async task failures, and external resource setup.
- Backend logs should use `tracing`; frontend logs should use the project's existing client-side logging mechanism or the smallest appropriate wrapper when none exists.
- Logs must be structured and actionable: include stable identifiers and module/kind names when they help debugging, but never log access tokens, passwords, secrets, or full sensitive payloads.
- Do not add noisy per-message `info` logs for hot paths such as media/datagram traffic; use debug-level diagnostics for expected high-frequency events and warn/error-level logs for unexpected failures.

## Configuration

- Local configuration is loaded from `.env`.
- Keep local database credentials in `.env`; do not commit local secrets or passwords.
- Do not add Docker Compose unless explicitly requested.

Использование субагентов - 10/10
