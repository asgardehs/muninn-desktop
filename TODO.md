# TODO

Running list of known follow-ups that aren't tied to a specific
open Phase of the rewrite. Phase scope lives in
[docs/rust-rewrite.md](docs/rust-rewrite.md).

## Licensing

- [ ] **Add LICENSE and NOTICE.** The NOTICE must name at minimum:
  - [Rhai](https://rhai.rs) — scripting engine
  - [harper-core](https://writewithharper.com/) — grammar checker
  - [axum](https://github.com/tokio-rs/axum) + [tokio](https://tokio.rs) + [tower-http](https://github.com/tower-rs/tower-http) — HTTP API (added in Phase 5)
  - [sqlparser-rs](https://github.com/apache/datafusion-sqlparser-rs) — SQL parsing
  - [comrak](https://github.com/kivikakk/comrak) — Markdown parser

  Sweep the full dependency tree when writing it — `cargo about` or
  `cargo-deny` can generate a complete list of crates and their
  licenses.

## Phase 5 follow-ups

These were carved out of Phase 5 scope — not blocking, but worth
tracking:

- [ ] Wire up the scripting engine's `runestone()` binding so scripts
  can evaluate Runestones directly (currently stubbed with a Phase 5
  error message that's now misleading — the module exists, just isn't
  bound)
- [ ] Multi-type Runestone sources (UNION-style; currently limited to
  one source type per Runestone)
- [ ] Surgical yaml-rust2 frontmatter edits in `runestones::update_cell`
  so cell writes preserve comments and formatting (currently preserves
  field order via `IndexMap` but drops comments)
- [ ] OpenAPI / schema export for the HTTP API so Huginn and Odin can
  generate clients
- [ ] Wikilink-aware JOIN resolution — currently joins require plain
  path strings in frontmatter; `[[wikilink]]` form needs a resolver
