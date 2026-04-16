---
project: Muninn
description:
  Design document for rewriting Muninn as a Rust + Tauri desktop application
  with React frontend, replacing the Go CLI + LSP architecture. mdbase
  specification adopted from the start.
doc_date: 2026-04-15
doc_rev_date: 2026-04-16
supersedes: design-doc.md
written_by: Adam Bick
---

# Muninn — Rust + Tauri Rewrite

_A personal knowledge base as a desktop application._
_"Odin's raven, who flies over the world gathering memory."_

## Overview

Muninn is rewritten from Go to Rust, gaining a desktop GUI via Tauri v2 with a
React frontend. The application manages a vault of markdown notes — creating,
searching, querying, and linking them — with a native UI instead of relying on
external editors and an LSP server.

The mdbase specification (v0.2.x) is adopted from the start rather than
retrofitted. Types, validation, and structured queries are built into the
foundation, not bolted on later.

**What ships:**

- **Desktop app** — Tauri v2 shell with React UI for browsing, editing,
  searching, and querying notes
- **Folder wikilinks** — `[[folder/]]` links to directories, folders as
  first-class nodes in the link graph with optional `_index.md` metadata
- **Runestones** — relational document views over the vault (Obsidian Bases-style
  spreadsheet interface over typed markdown)
- **Embedded scripting** — Rhai script blocks inside notes for custom queries,
  computed data, and dynamic content. No plugins needed.
- **Attachments** — images, PDFs, and files in `_attachments/` directories,
  embedded and linked from notes, tracked in the link graph
- **Export** — first-class document export via pandoc (PDF, HTML, DOCX, LaTeX,
  EPUB) with full Quarto (.qmd) support for computational documents
- **Grammar & spell checking** — harper-core embedded in the binary, real-time
  checking with vault-local custom dictionary
- **Zotero integration** — search Zotero library, export BibTeX/CSL
  bibliographies, citation picker in the editor via Zotero's HTTP citing
  protocol. Pandoc citation syntax (`[@citekey]`) throughout.
- **Internal API** — HTTP API for programmatic access from other Asgard tools
  (Huginn, Odin, scripts). Not a plugin ecosystem — internal use only.
- **CLI** — `muninn` binary for terminal workflows (create, search, query,
  run scripts, export, validate, manage types)

**What is dropped:**

- LSP server — editor integration replaced by native UI
- VS Code extension — no longer needed
- TextMate grammar — no longer needed
- Cobra, Goldmark, go.lsp.dev — all Go dependencies

Notes remain plain markdown files on disk. Git versioning still works.
Wikilinks still work. No database, no cloud.

## Architecture

```
┌───────────────────────────────────────────────────┐
│                  Tauri v2 Shell                    │
│  ┌─────────────────────────────────────────────┐  │
│  │            React Frontend (TS)              │  │
│  │                                             │  │
│  │  Editor ─ Search ─ Runestones ─ Graph ─ Query    │  │
│  └──────────────────┬──────────────────────────┘  │
│                     │ Tauri Commands (IPC)         │
│  ┌──────────────────▼──────────────────────────┐  │
│  │            Rust Backend                      │  │
│  │                                             │  │
│  │  ┌───────────────────────────────────────┐  │  │
│  │  │          muninn-core (lib)            │  │  │
│  │  │                                       │  │  │
│  │  │  vault/     — file I/O, search        │  │  │
│  │  │  mdbase/    — types, validation       │  │  │
│  │  │  query/     — SQL parser + evaluator  │  │  │
│  │  │  runestones/     — relational views        │  │  │
│  │  │  wikilink/  — extraction, index       │  │  │
│  │  │  markdown/  — parsing, frontmatter    │  │  │
│  │  │  scripting/ — Rhai engine + vault API │  │  │
│  │  │  export/    — pandoc + Quarto pipeline│  │  │
│  │  │  grammar/   — harper-core checker    │  │  │
│  │  │  zotero/    — citation management    │  │  │
│  │  │  watch/     — file system events      │  │  │
│  │  │  api/       — HTTP server (axum)      │  │  │
│  │  └───────────────────────────────────────┘  │  │
│  └─────────────────────────────────────────────┘  │
└───────────────────────────────────────────────────┘

  ┌──────────────────────────┐  ┌──────────────────────────┐
  │    muninn-cli (bin)      │  │   muninn-server (bin)    │
  │    clap commands         │  │   standalone API server  │
  │    calls muninn-core     │  │   calls muninn-core      │
  └──────────────────────────┘  └──────────────────────────┘

                                ┌──────────────────────────┐
                                │   Asgard consumers       │
                                │   Huginn, Odin, scripts  │
                                │   ──► HTTP API           │
                                └──────────────────────────┘
```

Four crates in a Cargo workspace:

| Crate | Type | Purpose |
|---|---|---|
| `muninn-core` | lib | All domain logic — vault, mdbase, query, runestones, scripting, export, grammar, zotero, wikilinks, markdown, API |
| `muninn-tauri` | bin | Tauri app — commands, window management, React frontend |
| `muninn-cli` | bin | Terminal interface — clap commands calling into core |
| `muninn-server` | bin | Standalone HTTP API server (for headless/service use) |

The core library is the single source of truth. The desktop app, CLI, and API
server are thin layers over it. No logic lives in the binary crates beyond
argument parsing, UI wiring, or HTTP routing.

The Tauri app embeds the API server on a local port at startup — other Asgard
tools connect to the running desktop app. `muninn-server` exists as a
standalone binary for headless environments (servers, CI, automation) where the
desktop app isn't running.

## Workspace Layout

```
muninn/
├── Cargo.toml                    # workspace root
├── crates/
│   ├── muninn-core/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── vault/
│   │       │   ├── mod.rs        # Vault struct, file I/O
│   │       │   ├── search.rs     # text search (title/tag/body scoring)
│   │       │   ├── list.rs       # filtered listing
│   │       │   └── tags.rs       # tag collection
│   │       ├── mdbase/
│   │       │   ├── mod.rs
│   │       │   ├── config.rs     # .muninn/config.yaml parsing
│   │       │   ├── types.rs      # TypeDef, FieldDef structs
│   │       │   ├── loader.rs     # load types from .muninn/types/
│   │       │   ├── validate.rs   # field validation
│   │       │   ├── match_type.rs # type matching (explicit, rules)
│   │       │   ├── inherit.rs    # type inheritance resolution
│   │       │   └── generate.rs   # generated field strategies
│   │       ├── query/
│   │       │   ├── mod.rs
│   │       │   ├── parser.rs     # SQL parsing (sqlparser-rs)
│   │       │   ├── ast.rs        # internal query representation
│   │       │   ├── eval.rs       # query execution against vault
│   │       │   ├── functions.rs  # built-in SQL functions
│   │       │   ├── value.rs      # typed query Value (SQL semantics)
│   │       │   └── computed.rs   # computed field evaluation (Phase 5)
│   │       ├── runestones/
│   │       │   ├── mod.rs
│   │       │   ├── runestone.rs   # Runestone definition, column config
│   │       │   ├── view.rs       # materialized view over vault
│   │       │   ├── relations.rs  # cross-runestone link traversal
│   │       │   └── storage.rs    # .muninn/runestones/*.yaml persistence
│   │       ├── api/
│   │       │   ├── mod.rs
│   │       │   ├── routes.rs     # axum route definitions
│   │       │   ├── handlers.rs   # request handlers → core calls
│   │       │   └── types.rs      # API request/response types
│   │       ├── wikilink/
│   │       │   ├── mod.rs
│   │       │   ├── extract.rs    # regex extraction from markdown
│   │       │   └── index.rs      # bidirectional link graph
│   │       ├── markdown/
│   │       │   ├── mod.rs
│   │       │   ├── parser.rs     # markdown splitting + frontmatter
│   │       │   ├── frontmatter.rs # YAML deserialization
│   │       │   └── heading.rs    # heading extraction
│   │       ├── scripting/
│   │       │   ├── mod.rs
│   │       │   ├── engine.rs     # Rhai engine setup + vault API registration
│   │       │   ├── functions.rs  # query(), search(), note(), table(), etc.
│   │       │   ├── render.rs     # execute script blocks within a note
│   │       │   └── loader.rs     # .muninn/scripts/*.rhai file loading
│   │       ├── export/
│   │       │   ├── mod.rs
│   │       │   ├── pipeline.rs   # resolve links → eval scripts → emit output
│   │       │   ├── pandoc.rs     # pandoc invocation + format options
│   │       │   ├── quarto.rs     # Quarto rendering (.qmd support)
│   │       │   └── formats.rs    # PDF, HTML, DOCX, LaTeX, EPUB config
│   │       ├── grammar/
│   │       │   ├── mod.rs
│   │       │   ├── checker.rs    # harper-core integration, diagnostic mapping
│   │       │   └── dictionary.rs # vault-local custom dictionary management
│   │       ├── zotero/
│   │       │   ├── mod.rs
│   │       │   ├── client.rs     # HTTP client for Zotero Web API v3 + local server
│   │       │   ├── types.rs      # ZoteroItem, Creator, Citation key types
│   │       │   └── picker.rs     # citation picker protocol (HTTP citing)
│   │       └── watch/
│   │           └── mod.rs        # file system watcher (notify crate)
│   ├── muninn-cli/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── cmd_note.rs       # note new/list/search/backlinks
│   │       ├── cmd_search.rs     # top-level search alias
│   │       ├── cmd_query.rs      # SQL queries
│   │       ├── cmd_type.rs       # type list/show
│   │       ├── cmd_validate.rs   # note validation
│   │       ├── cmd_export.rs     # export to PDF, HTML, DOCX, etc.
│   │       ├── cmd_cite.rs       # Zotero citation search, export
│   │       ├── cmd_init.rs       # vault setup
│   │       └── cmd_backfill.rs   # default/generated field backfill
│   ├── muninn-server/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── main.rs           # standalone API server entry point
│   └── muninn-tauri/
│       ├── Cargo.toml
│       ├── tauri.conf.json
│       ├── src/
│       │   ├── main.rs
│       │   └── commands/
│       │       ├── mod.rs
│       │       ├── vault.rs      # note CRUD, search, list
│       │       ├── query.rs      # SQL query execution
│       │       ├── types.rs      # type management
│       │       ├── wikilinks.rs  # link graph data
│       │       ├── export.rs     # export pipeline invocation
│       │       └── watch.rs      # file change events → frontend
│       └── frontend/
│           ├── package.json
│           ├── tsconfig.json
│           ├── vite.config.ts
│           └── src/
│               ├── App.tsx
│               ├── main.tsx
│               ├── views/
│               │   ├── Editor.tsx
│               │   ├── Search.tsx
│               │   ├── Runestones.tsx
│               │   ├── RunestoneView.tsx
│               │   ├── Query.tsx
│               │   ├── Graph.tsx
│               │   └── Types.tsx
│               ├── components/
│               │   ├── NoteList.tsx
│               │   ├── Frontmatter.tsx
│               │   ├── MarkdownPreview.tsx
│               │   ├── WikilinkChip.tsx
│               │   ├── RunestoneTable.tsx
│               │   ├── RunestoneCell.tsx
│               │   ├── RelationChip.tsx
│               │   └── QueryResults.tsx
│               └── lib/
│                   ├── tauri.ts   # invoke wrappers
│                   └── types.ts   # shared TS types
├── testdata/                      # vault fixtures for testing
├── docs/
├── Makefile
├── LICENSE
└── README.md
```

## muninn-core

### vault module

The `Vault` struct is the entry point for all file operations. It owns the
vault path, the loaded mdbase config, type definitions, and the wikilink index.

```rust
pub struct Vault {
    root: PathBuf,
    config: Option<MdbaseConfig>,
    types: HashMap<String, TypeDef>,
    wikilinks: Arc<RwLock<WikilinkIndex>>,
}

impl Vault {
    pub fn open(root: impl AsRef<Path>) -> Result<Self>;
    pub fn create_note(&self, title: &str, type_name: &str, fields: HashMap<String, Value>) -> Result<PathBuf>;
    pub fn read_note(&self, path: &Path) -> Result<Note>;
    pub fn list_notes(&self, filter: &NoteFilter) -> Result<Vec<NoteSummary>>;
    pub fn search(&self, query: &str, filter: Option<&NoteFilter>) -> Result<Vec<SearchResult>>;
    pub fn query(&self, sql: &str) -> Result<QueryResultSet>;
    pub fn validate(&self, path: &Path) -> Result<Vec<ValidationError>>;
    pub fn validate_all(&self) -> Result<Vec<(PathBuf, Vec<ValidationError>)>>;
    pub fn collect_tags(&self) -> Result<Vec<TagCount>>;
    pub fn backlinks(&self, path: &Path) -> Result<Vec<PathBuf>>;
    pub fn rename_note(&self, from: &Path, to: &str) -> Result<RenameResult>;
}
```

Thread safety: `Vault` is `Send + Sync`. The wikilink index is behind
`Arc<RwLock<>>`. File operations use path-level locking to prevent concurrent
writes to the same note. Read operations are lock-free (the filesystem is the
source of truth).

**Search** retains the current weighted scoring model:

- Title match: 3 points per query word
- Tag match: 2 points per query word
- Body match: 1 point per query word

Search walks the vault and scores against parsed notes. The `NoteFilter` struct
replaces hardcoded filter flags — it's built dynamically from available type
fields.

### mdbase module

Types, validation, and schema logic. Adopted from the start — no hardcoded
schema, no migration path from a pre-mdbase state.

Key types:

```rust
pub struct MdbaseConfig {
    pub spec_version: String,
    pub name: String,
    pub description: Option<String>,
    pub settings: Settings,
}

pub struct TypeDef {
    pub name: String,
    pub description: Option<String>,
    pub extends: Option<String>,
    pub fields: IndexMap<String, FieldDef>,
    pub computed: IndexMap<String, ComputedField>,
    pub match_rules: Vec<MatchRule>,
    pub strict: StrictMode,
    pub path_pattern: Option<String>,
}

pub struct FieldDef {
    pub field_type: FieldType,
    pub required: bool,
    pub default: Option<Value>,
    pub generated: Option<GeneratedStrategy>,
    pub description: Option<String>,
    pub deprecated: bool,
    pub unique: bool,
    pub constraints: Constraints,
}

pub enum FieldType {
    String,
    Integer,
    Number,
    Boolean,
    Date,
    DateTime,
    Time,
    Enum(Vec<String>),
    Any,
    List(Box<FieldDef>),
    Object(IndexMap<String, FieldDef>),
    Link { target: Vec<String>, validate_exists: bool },
}
```

`IndexMap` (from the `indexmap` crate) preserves field insertion order, matching
YAML source order for round-trip fidelity.

**Type loading** reads `.muninn/types/*.md`, parses frontmatter into `TypeDef`
structs, resolves inheritance via topological sort, and detects cycles.

**Validation** returns structured errors with mdbase error codes
(`missing_required`, `type_mismatch`, `constraint_violation`, etc.).

### query module

SQL query language over frontmatter fields, using `sqlparser-rs`.

```rust
pub fn parse_query(sql: &str) -> Result<MuninnQuery>;
pub fn execute(
    vault_root: &Path,
    types: &HashMap<String, TypeDef>,
    config: Option<&MdbaseConfig>,
    query: &MuninnQuery,
) -> Result<QueryResultSet>;

// Convenience wrapper on Vault
impl Vault {
    pub fn query(&self, sql: &str) -> Result<QueryResultSet>;
}
```

`sqlparser-rs` parses standard SQL into an AST. The `parser` module validates
that only supported clauses are used (SELECT, FROM, WHERE, GROUP BY, HAVING,
ORDER BY, LIMIT/OFFSET) and rejects DDL, DML, CTEs, UNION, and subqueries.

**FROM sources:** a single type name (matched via `mdbase::match_type`), or
the synthetic `note` source — `FROM note` selects every note in the vault
regardless of type.

The evaluator resolves FROM to matching notes, filters with WHERE, applies
GROUP BY/HAVING for aggregate queries, sorts (with projection-alias resolution
in ORDER BY), paginates, and projects selected fields. Built-in functions
(TODAY, NOW, YEAR, LENGTH, LOWER, UPPER, COALESCE, DATE_ADD, EXISTS) are
scalar; COUNT, SUM, AVG, MIN, MAX are aggregates.

A dedicated `query::Value` enum (distinct from `serde_yaml::Value`) gives the
evaluator real SQL semantics: tagged Date/DateTime values, integer/float
split, NULL-aware comparison. The round-trip is YAML → `Value` for evaluation,
`Value` → JSON for API/CLI output, `Value` → YAML for Runestone writeback
(Phase 5).

**JOINs** are deferred to Phase 5 where link-field semantics arrive with
Runestones. Until then the parser returns a clear error pointing to Phase 5.

**Computed fields** are introduced in Phase 5 alongside Runestone virtual
columns — the `computed.rs` module is a placeholder until `TypeDef` gains a
`computed:` map.

Resource limits: max result set size (10,000 rows), expression nesting depth
(32 levels). Evaluation timeout arrives alongside the persistent cache in
Phase 9.

### wikilink module

Regex extraction and bidirectional index. Extends the Go version with
first-class folder linking.

**Syntax:**

- `[[target]]` — link to a note
- `[[target|alias]]` — aliased link
- `[[target#heading]]` — link to heading within a note
- `[[folder/]]` — link to a folder (trailing slash)
- `[[folder/|alias]]` — aliased folder link

**Resolution:** Target normalized (lowercase, trimmed). Notes matched against
filenames without `.md`. Folders matched against directory names relative to
vault root.

**Folder wikilinks:**

This is the big one. Obsidian's most requested missing feature is the ability
to `[[link to a folder]]`. In Muninn, folders are first-class citizens in the
link graph.

A folder link resolves to the directory itself. Folders can optionally have an
`_index.md` file that provides metadata (frontmatter) and content for the
folder — similar to how static site generators treat `_index.md` or `index.md`.

```
notes/
├── projects/
│   ├── _index.md          # folder metadata + description
│   ├── plant-ops.md
│   └── safety-audit.md
├── journal/
│   ├── _index.md
│   └── 2026-04-W1-05.md
└── references/
    └── osha-standards.md  # no _index.md — folder still linkable
```

**How folders participate:**

| Feature | Behavior |
|---|---|
| Wikilinks | `[[projects/]]` links to the projects folder |
| Graph | Folders appear as nodes (distinct visual style from notes) |
| Backlinks | "What links to this folder?" works like note backlinks |
| Search | Folder `_index.md` frontmatter and body are searchable |
| Runestones | Folders with `_index.md` can appear as rows if they match a type |
| Navigation | Clicking a folder link opens a folder view (contents list + index preview) |
| Autocomplete | `[[` completion includes folder names with trailing `/` |

**Folder `_index.md` frontmatter:**

```yaml
---
title: Projects
type: folder
description: Active project workspaces
tags: [projects, work]
---

# Projects

Active project workspaces. Each project has its own note or subfolder.
```

Folders without `_index.md` are still linkable — they just have no metadata.
The link resolves to the directory, navigation shows the file listing, and the
folder appears in the graph as an untyped node.

**Type matching for folders:** A `folder` type can be defined in
`.muninn/types/folder.md` with its own schema. Folder `_index.md` files are
validated against it. This means folders can have structured metadata just like
notes — status, area, project, custom fields.

**Index structure:**

```rust
pub enum LinkTarget {
    Note(PathBuf),
    Folder(PathBuf),
    Heading { note: PathBuf, heading: String },
}

pub struct WikilinkIndex {
    forward: HashMap<PathBuf, Vec<LinkTarget>>,
    backlinks: HashMap<LinkTarget, Vec<PathBuf>>,
}
```

The index is bidirectional across both notes and folders. `Arc<RwLock<>>` for
thread safety, rebuilt from vault on startup.

Also extracts standard markdown links `[text](path.md)` and includes them in
the index.

### markdown module

Parsing uses `comrak` (full CommonMark + GFM, used by GitHub). Frontmatter
deserialization uses `serde_yaml`.

**Frontmatter preservation:** For writes, use `yaml-rust2` node-level API to
make surgical edits without round-tripping through serde. This preserves
comments, blank lines, quoting style, and field order — matching the mdbase
spec's round-trip requirement.

### watch module

File system watching using the `notify` crate (cross-platform: inotify on
Linux, FSEvents on macOS, ReadDirectoryChanges on Windows).

```rust
pub struct VaultWatcher {
    vault: Arc<Vault>,
    tx: broadcast::Sender<VaultEvent>,
}

pub enum VaultEvent {
    NoteCreated(PathBuf),
    NoteModified(PathBuf),
    NoteDeleted(PathBuf),
    NoteRenamed { from: PathBuf, to: PathBuf },
    TypeChanged(String),
    ConfigChanged,
}
```

Events are debounced (500ms) to handle bulk operations like `git checkout`.
The watcher updates the wikilink index and cache incrementally. In the Tauri
app, events are forwarded to the React frontend via Tauri's event system.

### scripting module

Embedded scripting engine for custom queries, data transformations, and dynamic
content inside notes. This replaces Obsidian's dependency on Dataview,
Templater, and CustomJS plugins with a single built-in capability.

**Runtime: Rhai**

[Rhai](https://rhai.rs) is a Rust-native embedded scripting language. It's the
natural choice because:

- No external runtime — compiles into the Muninn binary
- Sandboxed by default — no file system access, no network, no shell
- Rust-like syntax that's easy to pick up
- Functions and types from Rust are exposed directly to scripts
- Resource limits built in (max operations, max call depth, max string size)

**Inline script blocks:**

Scripts are written inside fenced code blocks with the `muninn` language tag.
They execute at render time and their output replaces the code block in the
preview.

````markdown
# Active Projects

```muninn
let notes = query("SELECT title, status, updated FROM note WHERE status = 'active' AND project IS NOT NULL ORDER BY updated DESC");
table(notes)
```

## Stats

```muninn
let total = query("SELECT COUNT(*) as count FROM note").first().count;
let active = query("SELECT COUNT(*) as count FROM note WHERE status = 'active'").first().count;
let draft = query("SELECT COUNT(*) as count FROM note WHERE status = 'draft'").first().count;

print(`Total notes: ${total}`);
print(`Active: ${active}`);
print(`Drafts: ${draft}`);
```
````

**Vault API exposed to scripts:**

```rust
// Functions registered into the Rhai engine
fn query(sql: &str) -> Vec<Map>           // SQL query, returns rows as maps
fn search(text: &str) -> Vec<Map>         // text search
fn note(path: &str) -> Map                // read a single note
fn notes(filter: Map) -> Vec<Map>         // list notes with filter
fn backlinks(path: &str) -> Vec<Map>      // get backlinks for a note
fn tags() -> Vec<Map>                     // all tags with counts
fn types() -> Vec<Map>                    // all type definitions
fn runestone(name: &str) -> Vec<Map>      // evaluate a named Runestone

// Output functions
fn table(rows: Vec<Map>)                  // render as markdown table
fn list(items: Vec<String>)               // render as bullet list
fn link(path: &str) -> String             // create a wikilink
fn print(text: &str)                      // raw text output
fn json(value: Dynamic) -> String         // JSON stringify
```

Scripts have read-only access to the vault. They cannot create, modify, or
delete notes. This is a deliberate constraint — scripts are for *viewing and
computing*, not for side effects. Note creation and mutation happen through
the UI, CLI, or API.

**Script files:**

For reusable logic, scripts can be stored as `.rhai` files in
`.muninn/scripts/`:

```
.muninn/
├── scripts/
│   ├── weekly-summary.rhai
│   ├── project-dashboard.rhai
│   └── tag-cloud.rhai
```

Referenced from notes:

````markdown
```muninn
import "weekly-summary"
```
````

**Execution model:**

- Scripts execute at render time in the desktop app (preview pane)
- Scripts execute on demand in the CLI (`muninn run script.rhai` or
  `muninn render note.md`)
- Scripts execute via API (`POST /api/render` returns note with scripts
  evaluated)
- Each script execution gets a fresh Rhai engine instance — no state persists
  between blocks or between renders
- Resource limits: max 1M operations, max 64 call stack depth, 5 second
  timeout

**Why not JavaScript?**

JavaScript would require embedding V8 or QuickJS — large dependencies with
complex build chains. Rhai compiles directly into the Rust binary with zero
external dependencies, starts instantly (no VM warmup), and is naturally
sandboxed. The target audience is the internal team, not a plugin ecosystem —
Rhai's simpler syntax is sufficient for data queries and formatting.

**What this replaces from Obsidian's plugin world:**

| Obsidian Plugin | Muninn Equivalent |
|---|---|
| Dataview (DQL queries) | `query()` function — same concept, SQL syntax |
| Dataview (inline fields) | mdbase typed frontmatter — no inline `key:: value` hack |
| DataviewJS (JS in notes) | `muninn` code blocks with Rhai |
| Templater (dynamic templates) | Scripts + type defaults + generated fields |
| CustomJS (shared JS functions) | `.muninn/scripts/*.rhai` imports |
| Tasks plugin (task queries) | `query("SELECT ... FROM task WHERE ...")` |

### attachments

Non-markdown files — images, PDFs, diagrams, spreadsheets — that live alongside
notes in the vault. Attachments are not notes. They have no frontmatter, don't
get typed, and don't appear in Runestones. But they are first-class citizens in the
link graph.

**Directory convention:**

Each folder in the vault can have an `_attachments/` subdirectory:

```
notes/
├── projects/
│   ├── plant-ops.md
│   ├── _attachments/
│   │   ├── site-map.png
│   │   ├── inspection-report.pdf
│   │   └── flow-diagram.svg
│   └── _index.md
├── references/
│   ├── osha-standards.md
│   └── _attachments/
│       └── osha-table-z1.pdf
└── _attachments/               # vault-root attachments
    └── logo.png
```

Attachments live near the notes that reference them. This keeps related files
co-located, works naturally with git, and makes the vault self-contained when
moved or shared.

**Referencing attachments:**

- `![[site-map.png]]` — embed an image (rendered inline in editor preview)
- `[[inspection-report.pdf]]` — link to a file (clickable, opens externally)
- `![[flow-diagram.svg]]` — embed an SVG inline

Resolution searches `_attachments/` directories from the current note's folder
upward to vault root, matching by filename. Explicit paths also work:
`![[projects/_attachments/site-map.png]]`.

**How attachments participate:**

| Feature | Behavior |
|---|---|
| Wikilinks | `[[file.pdf]]` and `![[image.png]]` resolve to attachments |
| Graph | Attachment nodes shown with file-type icon (optional, can be toggled) |
| Backlinks | "What notes reference this attachment?" |
| Search | Searchable by filename, not by content |
| Export | Embedded images included in PDF/HTML export; linked files referenced |
| Validation | Broken attachment references reported as diagnostics |
| File watching | New/deleted attachments update the index in real time |

**Embed syntax:**

The `!` prefix distinguishes embeds from links — same convention as Obsidian:

- `![[image.png]]` — render the image inline in the preview
- `![[diagram.svg]]` — render SVG inline
- `![[note.md]]` — embed another note's content (transclusion)
- `![[note.md#heading]]` — embed a specific section

**What attachments are NOT:**

- Not typed or validated (no frontmatter, no mdbase schema)
- Not queryable via SQL (they're not rows in any table)
- Not editable in Muninn (images open in the system viewer, PDFs in the system
  PDF reader)
- Not indexed for content search (searching inside PDFs is a different problem)

The `_attachments/` directories are excluded from note listing and type
matching, same as `.muninn/`. They're part of the vault but not part of the
knowledge graph beyond their link relationships.

**LinkTarget extension:**

```rust
pub enum LinkTarget {
    Note(PathBuf),
    Folder(PathBuf),
    Heading { note: PathBuf, heading: String },
    Attachment(PathBuf),
}
```

Adding `Attachment` as a variant means the compiler enforces handling across
the wikilink index, graph rendering, backlinks, and validation — same
structural safety as folder links.

### export module

First-class document export via pandoc and Quarto. Notes are authored in
Muninn, published as PDF, HTML, DOCX, LaTeX, or EPUB. Quarto support makes
Muninn viable for academic and data science workflows where computational
documents need to render to publication-quality output.

**Export pipeline:**

```
Note (.md / .qmd)
  │
  ├─ 1. Resolve wikilinks → relative links or inline content
  ├─ 2. Resolve attachment embeds → image paths / inline SVG
  ├─ 3. Evaluate Rhai script blocks → static markdown output
  ├─ 4. Preserve Quarto code blocks ({python}, {r}, {julia}) as-is
  │
  ▼
Processed Markdown
  │
  ├─ pandoc path: pandoc --from markdown --to <format>
  └─ Quarto path: quarto render <file> --to <format>
  │
  ▼
Output (PDF, HTML, DOCX, LaTeX, EPUB)
```

The pipeline runs inside `muninn-core`. Pandoc and Quarto are external
dependencies — Muninn invokes them as subprocesses. If neither is installed,
export commands fail with a clear message pointing to installation instructions.

**Pandoc export (standard markdown notes):**

```rust
pub struct ExportOptions {
    pub format: ExportFormat,
    pub template: Option<PathBuf>,     // custom pandoc template
    pub css: Option<PathBuf>,          // for HTML export
    pub bibliography: Option<PathBuf>, // BibTeX/CSL for citations
    pub csl: Option<PathBuf>,          // citation style
    pub toc: bool,                     // table of contents
    pub number_sections: bool,
    pub output: Option<PathBuf>,       // output path (default: same name, new ext)
}

pub enum ExportFormat {
    Pdf,
    Html,
    Docx,
    Latex,
    Epub,
    Markdown,  // processed markdown (links resolved, scripts evaluated)
}

impl Vault {
    pub fn export(&self, path: &Path, options: &ExportOptions) -> Result<PathBuf>;
    pub fn export_batch(&self, paths: &[PathBuf], options: &ExportOptions) -> Result<Vec<PathBuf>>;
}
```

**What happens during export:**

1. **Wikilinks resolved** — `[[some-topic]]` becomes `[Some Topic](some-topic.md)`
   for HTML/EPUB or a cross-reference for PDF/LaTeX. Broken links become
   plain text with a warning.
2. **Attachment embeds resolved** — `![[diagram.png]]` becomes
   `![](projects/_attachments/diagram.png)` with the correct relative path.
   Images are included in the output. Non-image attachments become download
   links (HTML) or footnote references (PDF).
3. **Rhai script blocks evaluated** — `muninn` code fences are replaced with
   their output. A query that returns a table becomes a markdown table in the
   exported document. This is the same evaluation as the preview pane —
   export produces what you see.
4. **Frontmatter preserved or transformed** — pandoc reads YAML frontmatter
   natively for title, author, date. Additional mdbase fields are stripped
   or passed through depending on the format.

**Quarto support (.qmd files):**

Quarto files are markdown with executable code blocks — `{python}`, `{r}`,
`{julia}`, `{observable}`. Muninn treats `.qmd` as a first-class note type.

```yaml
# .muninn/types/quarto.md
---
name: quarto
extends: note
match_rules:
  - path_glob: "**/*.qmd"
fields:
  format:
    type: enum
    values: [pdf, html, docx, revealjs, beamer]
    default: pdf
  bibliography:
    type: string
  csl:
    type: string
  execute:
    type: object
    fields:
      echo:
        type: boolean
        default: true
      warning:
        type: boolean
        default: true
---

# Quarto Document

Computational document with executable code blocks. Renders to publication-
quality output via Quarto.
```

**What `.qmd` gets in Muninn:**

- Full syntax highlighting in the editor for `{python}`, `{r}`, `{julia}`
  code blocks (CodeMirror language nesting)
- Wikilinks work inside `.qmd` files — link to other notes, reference
  attachments, embed content
- Frontmatter validated against the `quarto` type — Quarto-specific fields
  (format, bibliography, execute options) coexist with mdbase fields
- Appears in Runestones, searchable, part of the link graph — it's a note
- Rhai blocks and Quarto blocks coexist: `muninn` fences query the vault,
  `{python}` fences run Python. No ambiguity.
- Export: `muninn export paper.qmd` calls `quarto render` under the hood

**The academic workflow:**

A researcher using Muninn:

1. Organizes literature notes, methodology notes, data notes in the vault
2. Links them together with wikilinks — builds a knowledge graph of their
   research
3. Uses Rhai script blocks to generate dynamic tables ("all notes tagged
   'experiment' with status 'complete'")
4. Writes their paper as a `.qmd` file in the same vault, with `{python}`
   blocks for analysis and `{r}` blocks for figures
5. References other notes via wikilinks — during export, these resolve to
   proper cross-references or inline content
6. Exports to PDF via Quarto with `muninn export paper.qmd --format pdf`
7. The PDF includes rendered code output, resolved references, embedded
   figures from `_attachments/`, and a bibliography

All of this happens in one application. No switching between Obsidian for
notes, VS Code for Quarto, and a terminal for rendering.

**CLI commands:**

```bash
# Export single note
muninn export path/to/note.md --format pdf
muninn export path/to/note.md --format html --css custom.css
muninn export paper.qmd --format pdf

# Export with options
muninn export paper.qmd --format pdf --bibliography refs.bib --csl apa.csl --toc

# Export multiple notes
muninn export --type design-doc --format pdf --output ./exports/

# Export to processed markdown (links resolved, scripts evaluated)
muninn export path/to/note.md --format markdown
```

**Desktop app integration:**

The editor toolbar includes an "Export" button with a dropdown for format
selection. Export settings are remembered per note (stored in app config, not
in the note). A progress indicator shows pandoc/Quarto execution status. The
exported file opens automatically in the system viewer.

For `.qmd` files, the editor shows a "Render" button alongside "Export" — same
as Quarto's own UI convention. Render previews in-app (HTML output displayed
in a webview panel), Export produces a file.

**API endpoints:**

```
POST   /api/export/:path          # export a note (body: ExportOptions)
GET    /api/export/formats        # list available formats (checks pandoc/quarto)
```

### grammar module

Built-in grammar and spell checking via `harper-core` — the Rust library
behind harper-ls. Embedded directly as a crate dependency, not as a subprocess
or external tool. The grammar engine runs in-process with zero startup cost.

**Why harper-core, not harper-ls:**

Harper ships as two crates: `harper-ls` (the Language Server Protocol wrapper)
and `harper-core` (the grammar engine). Since Muninn dropped the LSP
architecture, we use `harper-core` directly. The grammar engine is called from
Rust code — no protocol overhead, no stdio transport, no JSON-RPC
serialization. Just function calls.

This also means harper's checking is available everywhere — the desktop app,
the CLI (`muninn lint`), and the API (`POST /api/lint/:path`) — because it's
part of `muninn-core`, not a separate process that only the editor talks to.

**What harper provides:**

- Spell checking with a built-in English dictionary
- Grammar rule checking (subject-verb agreement, article usage, common
  mistakes, passive voice detection, etc.)
- Markdown-aware — understands that code blocks, frontmatter, and wikilinks
  are not prose and should be skipped
- Suggestions for corrections
- Fast enough for real-time checking as the user types

**Integration with Muninn:**

```rust
pub struct GrammarChecker {
    harper: harper_core::LintGroup,
    custom_dict: Dictionary,
}

pub struct GrammarDiagnostic {
    pub span: Range<usize>,
    pub message: String,
    pub suggestions: Vec<String>,
    pub severity: DiagnosticSeverity,  // error (misspelling) or warning (grammar)
    pub rule: String,                  // harper rule identifier
}

impl GrammarChecker {
    pub fn new(vault_dict_path: Option<&Path>) -> Result<Self>;
    pub fn check(&self, content: &str) -> Vec<GrammarDiagnostic>;
    pub fn add_to_dictionary(&mut self, word: &str) -> Result<()>;
}
```

The checker operates on note body content only — frontmatter YAML, code fences
(including `muninn` and `{python}` blocks), and wikilink syntax are excluded
from checking. Harper's built-in markdown parsing handles most of this;
Muninn extends it to also skip wikilink targets and Quarto-specific syntax.

**Custom dictionary:**

Each vault has an optional custom dictionary at `.muninn/dictionary.txt` — one
word per line. This handles domain-specific terminology (EHS jargon, project
names, technical terms) that the default dictionary doesn't know.

```
~/.local/share/muninn/
├── .muninn/
│   ├── dictionary.txt         # vault-specific words
│   ├── config.yaml
│   └── ...
```

In the desktop app, right-clicking a flagged word shows "Add to dictionary" —
writes to `.muninn/dictionary.txt` and immediately suppresses the diagnostic.
The dictionary file is plain text, git-trackable, and shared across the vault.

**Desktop app integration:**

- Red squiggly underlines for spelling errors in the CodeMirror editor
- Blue squiggly underlines for grammar suggestions
- Right-click context menu: suggested corrections, "Add to dictionary",
  "Ignore rule"
- Grammar panel in sidebar: list of all diagnostics in the current note,
  click to navigate
- Toggle: grammar checking can be disabled per note type (e.g. skip checking
  on notes of type `code-snippet`) via config

**CLI integration:**

```bash
# Check a single note
muninn lint path/to/note.md

# Check all notes
muninn lint

# Check notes of a specific type
muninn lint --type design-doc

# Output formats
muninn lint --format json           # machine-readable
muninn lint --format pretty         # colored terminal output (default)
```

**API integration:**

```
POST   /api/lint/:path              # check a note, returns diagnostics
POST   /api/dictionary              # add word to custom dictionary
```

**Configuration:**

Grammar settings in `.muninn/config.yaml`:

```yaml
grammar:
  enabled: true
  language: "en-US"
  skip_types: [code-snippet]    # note types to skip checking
  disabled_rules: []            # specific harper rules to suppress
```

### zotero module

Citation management via Zotero integration. Muninn talks to Zotero through two
official APIs — the Web API v3 for library access and the local HTTP citing
protocol for interactive citation insertion. No direct SQLite access (Zotero
docs explicitly warn against it — schema changes between releases and writes
corrupt the database).

This module is designed against **Zotero 9.0** (April 2026) but targets stable
APIs that work across Zotero 7, 8, and 9. The Web API v3 has been stable since
2022. The HTTP citing protocol on port 23119 has been the official integration
path since Zotero 6.0 (used by the Google Docs plugin).

**Two integration paths:**

| Path | Endpoint | Auth | Use Case |
|---|---|---|---|
| Web API v3 | `https://api.zotero.org` | API key via `Zotero-API-Key` header | Search library, export BibTeX/CSL-JSON, read items |
| HTTP Citing Protocol | `http://127.0.0.1:23119/connector/document/*` | None (localhost only) | Interactive citation picker in desktop app |

**Web API v3 — library access:**

The primary integration for CLI and export workflows. Zotero's REST API
supports searching items, retrieving metadata, and exporting in all standard
formats:

- `bibtex`, `biblatex` — for pandoc `--bibliography`
- `csljson` — Citation Style Language JSON, for programmatic citation data
- `ris`, `mods`, `tei` — additional academic formats

Items are accessed via user or group library URLs:

```
GET /users/<userID>/items?q=<search>&format=bibtex
GET /users/<userID>/items/<itemKey>?format=csljson
GET /users/<userID>/items?tag=<tag>&format=bibtex
```

Authentication uses an API key created in Zotero account settings. The key is
stored in `.muninn/config.yaml` (vault-portable) or app config (machine-local).

**HTTP Citing Protocol — interactive picker:**

For the Tauri desktop app, the HTTP citing protocol lets Muninn trigger
Zotero's native citation dialog. The flow is a transaction-based exchange:

1. Muninn sends `POST /connector/document/execCommand` with
   `{"command": "addEditCitation", "docId": "<noteId>"}`
2. Zotero responds with commands (get document state, show picker)
3. Muninn responds to each command via `POST /connector/document/respond`
4. Transaction completes when Zotero issues `Document.complete`

This is the same protocol Google Docs uses. Muninn implements it as a document
processor — the note is the "document", citations are inserted as pandoc-style
`[@citekey]` references.

**Citation syntax in notes:**

Muninn uses pandoc citation syntax — the same format Quarto and pandoc expect:

- `[@smith2024]` — single citation
- `[@smith2024; @jones2023]` — multiple citations
- `[@smith2024, p. 42]` — citation with locator
- `[see @smith2024, pp. 33-35; also @jones2023, ch. 1]` — complex citation

These are plain text in the markdown body. They render through pandoc/Quarto
during export with the appropriate `--bibliography` and `--csl` flags.

**What the module provides:**

```rust
pub struct ZoteroClient {
    api_key: Option<String>,
    user_id: Option<String>,
    base_url: String,            // https://api.zotero.org (configurable)
    local_port: u16,             // 23119 (default)
}

pub struct ZoteroItem {
    pub key: String,             // Zotero item key
    pub item_type: String,       // book, journalArticle, thesis, etc.
    pub title: String,
    pub creators: Vec<Creator>,
    pub date: Option<String>,
    pub abstract_text: Option<String>,
    pub tags: Vec<String>,
    pub cite_key: Option<String>, // Better BibTeX key if available
}

pub struct Creator {
    pub creator_type: String,    // author, editor, translator, etc.
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub name: Option<String>,    // for institutional authors
}

impl ZoteroClient {
    pub fn new(config: &ZoteroConfig) -> Self;
    pub async fn search(&self, query: &str) -> Result<Vec<ZoteroItem>>;
    pub async fn get_item(&self, key: &str) -> Result<ZoteroItem>;
    pub async fn export_bibliography(&self, keys: &[&str], format: BibFormat) -> Result<String>;
    pub async fn export_collection(&self, collection: &str, format: BibFormat) -> Result<String>;
    pub fn is_local_available(&self) -> bool;  // ping port 23119
}

pub enum BibFormat {
    BibTeX,
    BibLaTeX,
    CslJson,
    Ris,
}
```

**Integration with the export pipeline:**

The export module's `ExportOptions.bibliography` field currently expects a file
path. With Zotero integration, this gains a second mode:

- `bibliography: "refs.bib"` — existing behavior, local file
- `bibliography: "zotero:collection/My Research"` — pull from Zotero on export
- `bibliography: "zotero:items"` — export all cited items (scan note for `[@...]`)

When a note is exported and bibliography is set to a Zotero source, the export
pipeline calls `ZoteroClient::export_bibliography()` to generate a temporary
`.bib` file, then passes it to pandoc/Quarto.

**Integration with the desktop app:**

In the Tauri GUI:

- `Ctrl+Shift+Z` — open citation picker (triggers HTTP citing protocol)
- Citation autocomplete in CodeMirror: typing `[@` shows suggestions from
  Zotero search
- Bibliography panel in sidebar: browse Zotero collections, drag-and-drop
  citations
- Export dialog: "Pull bibliography from Zotero" checkbox

**CLI commands:**

```bash
muninn cite search "smith 2024"              # search Zotero library
muninn cite export --format bibtex           # export all cited items as .bib
muninn cite export --collection "My Research" --format biblatex
muninn cite key smith2024                    # look up a specific citation key
```

**Configuration:**

Zotero settings in `.muninn/config.yaml`:

```yaml
zotero:
  enabled: true
  api_key: "P9NiFoyLeZu2bZNvvuQPDWsd"  # from zotero.org/settings/keys
  user_id: "12345678"                    # Zotero user ID
  local_port: 23119                      # default Zotero connector port
  default_format: bibtex                 # bibtex | biblatex | csljson
```

The API key and user ID are required for Web API access. Local HTTP citing
(port 23119) works without authentication — it only requires Zotero to be
running on the same machine.

**What Zotero integration is NOT:**

- Not a Zotero replacement — Muninn manages notes, Zotero manages references
- Not a sync mechanism — Muninn pulls from Zotero on demand, doesn't cache
- Not a Zotero plugin — Muninn is a client of Zotero's APIs, not an extension
  that runs inside Zotero
- Not required — Zotero integration is optional. Notes with `[@citekey]`
  syntax work with any `.bib` file, Zotero just makes sourcing them easier

### runestones module

Runestones are relational document views over the vault — a spreadsheet interface
where types are tables, notes are rows, and frontmatter fields are columns.
This is the primary way users interact with structured data in Muninn, inspired
by Obsidian Runestones.

**What a Runestone is:**

A Runestone is a saved, named view definition stored in `.muninn/runestones/`.
Each Runestone targets one or more types and defines which columns to show, how
to sort, how to filter, and how to group. The underlying data is always the
vault — a Runestone is a lens, not a copy.

```yaml
# .muninn/runestones/active-work.yaml
name: "Active Work"
description: "All active notes across projects"
source:
  types: [note, design-doc, decision]
  filter: "status = 'active'"
columns:
  - field: title
    width: 300
  - field: type
    width: 100
  - field: project
    width: 150
  - field: status
    width: 100
  - field: tags
    width: 200
  - field: updated
    width: 120
    sort: desc
group_by: project
```

**Core types:**

```rust
pub struct Runestone {
    pub name: String,
    pub description: Option<String>,
    pub source: RunestoneSource,
    pub columns: Vec<ColumnDef>,
    pub group_by: Option<String>,
}

pub struct RunestoneSource {
    pub types: Vec<String>,
    pub filter: Option<String>,  // SQL WHERE clause
}

pub struct ColumnDef {
    pub field: String,
    pub width: Option<u32>,
    pub sort: Option<SortDirection>,
    pub hidden: bool,
    pub computed: Option<String>,  // SQL expression for virtual columns
}

pub struct RunestoneRow {
    pub path: PathBuf,
    pub cells: IndexMap<String, Value>,
}

impl Runestone {
    pub fn load(path: &Path) -> Result<Self>;
    pub fn evaluate(&self, vault: &Vault) -> Result<Vec<RunestoneRow>>;
    pub fn update_cell(&self, vault: &Vault, path: &Path, field: &str, value: Value) -> Result<()>;
}
```

**Inline editing:** When a user edits a cell in the Runestone table, the change
writes directly to that note's frontmatter. The `update_cell` method uses the
same surgical YAML editing (yaml-rust2 node API) as the rest of the system —
only the target field is modified, everything else preserved.

**Relations between Runestones:**

Link fields (`type: link`) in mdbase types create navigable relations between
notes. In a Runestone view, link columns render as clickable chips showing the
linked note's title. Clicking opens that note or jumps to the related Runestone
filtered to that record.

```yaml
# Type definition with relations
fields:
  supersedes:
    type: link
    target: note
  related:
    type: list
    items:
      type: link
      target: [note, reference]
```

In the Runestone UI, these render as relation columns — similar to foreign key
columns in a database GUI. Users can:

- Click a relation chip to navigate to the linked note
- Add/remove links via a dropdown that searches by title
- See reverse relations (what links *to* this note) as a virtual column

**Relation traversal in Runestones:**

Runestones can pull fields from linked notes into their columns using dot notation:

```yaml
columns:
  - field: title
  - field: supersedes.title    # title of the linked note
  - field: supersedes.status   # status of the linked note
```

This maps to the SQL JOIN semantics already in the query engine — `supersedes`
is resolved via the link, then `.title` accesses the linked note's frontmatter.

**Computed columns:**

Runestones support virtual columns defined as SQL expressions:

```yaml
columns:
  - field: _age
    computed: "DATE_ADD(created, TODAY())"
  - field: _link_count
    computed: "LENGTH(file.backlinks())"
```

Computed columns are read-only and evaluated at render time.

**Storage:** Runestone definitions live in `.muninn/runestones/*.yaml`. They are
user-editable files, same as type definitions. The desktop app provides a UI
for creating and configuring Runestones, but they can also be written by hand or
generated by scripts via the API.

### api module

HTTP API for programmatic access to the vault. Internal to the Asgard
ecosystem — used by Huginn, Odin, and automation scripts. Not a public plugin
API; no auth, no rate limiting, no versioned stability guarantees beyond what
the team needs.

Built with `axum` (Tokio-based, same async runtime as the rest of the app).

**Endpoints:**

```
# Notes
GET    /api/notes                    # list notes (query params for filtering)
POST   /api/notes                    # create note
GET    /api/notes/:path              # read note (frontmatter + body)
PUT    /api/notes/:path              # update note
DELETE /api/notes/:path              # delete note

# Search
GET    /api/search?q=:query          # text search
POST   /api/query                    # SQL query (body: { "sql": "..." })

# Runestones
GET    /api/runestones                # list runestones
GET    /api/runestones/:name              # evaluate runestone (returns rows)
PUT    /api/runestones/:name/rows/:path   # update a cell in a runestone row

# Types
GET    /api/types                    # list type definitions
GET    /api/types/:name              # get type schema

# Wikilinks
GET    /api/links/graph              # full link graph (nodes + edges)
GET    /api/links/backlinks/:path    # backlinks for a note

# Scripting
POST   /api/render/:path             # render note with script blocks evaluated
POST   /api/run                      # execute ad-hoc Rhai script (body: { "code": "..." })

# Export
POST   /api/export/:path             # export note (body: ExportOptions)
GET    /api/export/formats            # available formats (checks pandoc/quarto)

# Validation
POST   /api/validate                 # validate note(s)

# Meta
GET    /api/health                   # server status
GET    /api/schema                   # OpenAPI-style schema discovery
```

All responses are JSON. The `/api/schema` endpoint returns a self-describing
schema derived from loaded mdbase types — callers can discover available types,
fields, and constraints without hardcoding them.

**Embedding in Tauri:**

The Tauri app starts an axum server on a local port (default: `localhost:9200`)
at launch. This means other Asgard tools can talk to the running Muninn
instance without a separate server process.

```rust
// In muninn-tauri main.rs
let vault = Arc::new(Vault::open(&vault_path)?);
let api = muninn_core::api::router(vault.clone());

// Start API server in background
tokio::spawn(async move {
    axum::serve(
        tokio::net::TcpListener::bind("127.0.0.1:9200").await.unwrap(),
        api,
    ).await
});
```

**Standalone server:**

`muninn-server` is the same API without the Tauri shell — for headless use:

```bash
muninn-server --vault ~/.local/share/muninn --port 9200
```

Use cases: running on a home server, CI pipelines querying the vault,
automation scripts that don't need the GUI.

**Example: Huginn querying Muninn:**

```python
# Huginn pulling context from the knowledge base
import requests

# Find all active design docs
resp = requests.post("http://localhost:9200/api/query", json={
    "sql": "SELECT title, status, project FROM design-doc WHERE status = 'active'"
})
notes = resp.json()["rows"]

# Read a specific note's full content
resp = requests.get("http://localhost:9200/api/notes/some-topic.md")
note = resp.json()  # { frontmatter: {...}, body: "..." }
```

**Example: Script creating notes:**

```bash
# Automation: create a new incident note
curl -X POST http://localhost:9200/api/notes \
  -H "Content-Type: application/json" \
  -d '{
    "title": "Pump Station 7 Overflow",
    "type": "incident",
    "fields": {
      "status": "active",
      "area": "work",
      "project": "plant-ops",
      "tags": ["incident", "overflow", "ps7"]
    }
  }'
```

## muninn-tauri

### Tauri Commands

Each Tauri command is a thin wrapper that calls `muninn-core` and returns
serializable results. Commands are async (Tokio runtime).

```rust
#[tauri::command]
async fn list_notes(
    vault: State<'_, Arc<Vault>>,
    filter: Option<NoteFilter>,
) -> Result<Vec<NoteSummary>, String>;

#[tauri::command]
async fn read_note(
    vault: State<'_, Arc<Vault>>,
    path: String,
) -> Result<Note, String>;

#[tauri::command]
async fn create_note(
    vault: State<'_, Arc<Vault>>,
    title: String,
    type_name: String,
    fields: HashMap<String, Value>,
) -> Result<String, String>;

#[tauri::command]
async fn save_note(
    vault: State<'_, Arc<Vault>>,
    path: String,
    content: String,
) -> Result<(), String>;

#[tauri::command]
async fn search(
    vault: State<'_, Arc<Vault>>,
    query: String,
    filter: Option<NoteFilter>,
) -> Result<Vec<SearchResult>, String>;

#[tauri::command]
async fn execute_query(
    vault: State<'_, Arc<Vault>>,
    sql: String,
) -> Result<QueryResultSet, String>;

#[tauri::command]
async fn get_backlinks(
    vault: State<'_, Arc<Vault>>,
    path: String,
) -> Result<Vec<BacklinkInfo>, String>;

#[tauri::command]
async fn get_graph(
    vault: State<'_, Arc<Vault>>,
) -> Result<GraphData, String>;

#[tauri::command]
async fn list_types(
    vault: State<'_, Arc<Vault>>,
) -> Result<Vec<TypeSummary>, String>;

#[tauri::command]
async fn validate_note(
    vault: State<'_, Arc<Vault>>,
    path: String,
) -> Result<Vec<ValidationError>, String>;

// Scripting
#[tauri::command]
async fn render_note(
    vault: State<'_, Arc<Vault>>,
    path: String,
) -> Result<RenderedNote, String>;  // note with script blocks evaluated

#[tauri::command]
async fn run_script(
    vault: State<'_, Arc<Vault>>,
    code: String,
) -> Result<String, String>;  // ad-hoc script execution

// Grammar
#[tauri::command]
async fn lint_note(
    vault: State<'_, Arc<Vault>>,
    path: String,
) -> Result<Vec<GrammarDiagnostic>, String>;

#[tauri::command]
async fn add_to_dictionary(
    vault: State<'_, Arc<Vault>>,
    word: String,
) -> Result<(), String>;

// Export
#[tauri::command]
async fn export_note(
    vault: State<'_, Arc<Vault>>,
    path: String,
    options: ExportOptions,
) -> Result<PathBuf, String>;

#[tauri::command]
async fn available_export_formats(
    vault: State<'_, Arc<Vault>>,
) -> Result<Vec<ExportFormat>, String>;  // checks if pandoc/quarto are installed

// Runestones
#[tauri::command]
async fn list_runestones(vault: State<'_, Arc<Vault>>) -> Result<Vec<RunestoneSummary>, String>;

#[tauri::command]
async fn evaluate_runestone(
    vault: State<'_, Arc<Vault>>,
    name: String,
) -> Result<Vec<RunestoneRow>, String>;

#[tauri::command]
async fn update_runestone_cell(
    vault: State<'_, Arc<Vault>>,
    runestone_name: String,
    note_path: String,
    field: String,
    value: Value,
) -> Result<(), String>;

#[tauri::command]
async fn create_runestone(
    vault: State<'_, Arc<Vault>>,
    runestone: Runestone,
) -> Result<(), String>;
```

### Tauri Events

File system changes are pushed to the frontend as Tauri events:

```rust
// Backend emits
app.emit("vault-changed", VaultEvent::NoteModified(path));

// Frontend listens
listen("vault-changed", (event) => {
    // refresh note list, re-render editor, update graph
});
```

This replaces the LSP's `textDocument/didChange` notifications with something
the React frontend can consume directly.

### React Frontend

The frontend is a single-window app with a sidebar + main content layout.

**Views:**

| View | Purpose |
|---|---|
| **Editor** | Markdown editor (CodeMirror 6) with live preview, wikilink autocomplete, frontmatter form |
| **Search** | Text search with type/field filters, ranked results |
| **Runestones** | Relational spreadsheet views over typed notes — the primary structured data interface |
| **Query** | SQL input with table/JSON output, saved queries |
| **Graph** | Force-directed graph of wikilink connections — notes, folders, and attachments (D3 or vis-network) |
| **Types** | Browse and manage mdbase type definitions |
| **Folder** | Folder contents view when navigating to a `[[folder/]]` link — file list + `_index.md` preview |

**Editor details:**

The editor is the primary view. It combines:

- CodeMirror 6 with markdown syntax highlighting
- Custom wikilink decoration (clickable, color-coded for resolved/broken)
- Autocomplete for wikilinks triggered by `[[`, tag values, and enum fields
- Frontmatter rendered as a form above the editor (fields driven by matched
  type definition — dropdowns for enums, date pickers for dates, tag chips
  for lists)
- Split pane: editor left, rendered preview right (toggleable)
- `muninn` code blocks render inline in preview — script output replaces the
  code fence (tables, lists, computed values)
- `![[image.png]]` embeds render inline in preview — images from `_attachments/`
- Folder wikilinks render with a folder icon, distinct from note links
- `.qmd` files get syntax highlighting for `{python}`, `{r}`, `{julia}` blocks
- Real-time grammar/spell checking — red underlines (spelling), blue underlines
  (grammar), right-click for suggestions and "Add to dictionary"
- Export button in toolbar — format dropdown, remembers settings per note
- Backlinks panel below or in sidebar

This brings all the LSP capabilities into the native UI:

| Former LSP Feature | Desktop Equivalent |
|---|---|
| Completions | CodeMirror autocomplete extensions |
| Go to Definition | Click wikilink → navigate to note |
| References (backlinks) | Backlinks panel |
| Hover (note preview) | Tooltip on wikilink hover |
| Diagnostics | Inline markers + problems panel |
| Semantic Tokens | CodeMirror decorations for link state |
| Code Actions | Context menu (create missing note, etc.) |
| Rename | Rename dialog with cascading updates |
| Code Lens | Inline reference counts |

**Runestones view details:**

The Runestones view is the relational database interface. It renders Runestone definitions
as interactive spreadsheet tables.

- Column headers from Runestone column definitions, reorderable via drag
- Rows are notes matching the Runestone's source types and filter
- Cells are editable inline — changes write to frontmatter immediately
- Enum fields render as dropdowns, dates as date pickers, tags as chip inputs,
  links as searchable relation selectors
- Sort by clicking column headers, filter via a toolbar
- Group by a field to create collapsible sections
- "New row" button creates a note of the Runestone's source type with defaults
- Relation columns show linked note titles as clickable chips
- Computed columns are read-only with a visual indicator
- "New Runestone" wizard: pick source types, select columns, set initial filters
- Each Runestone is a tab — multiple Runestones can be open simultaneously

The underlying data is always the vault. The Runestone view is reactive — file
system events (via the watch module) trigger row updates in real time.

**Graph view details:**

The wikilink index provides all the data. The frontend receives a `GraphData`
struct (nodes + edges) and renders it with D3 force simulation or vis-network.
Nodes are notes *and folders* — folders have a distinct visual style (e.g.
different shape or color) to distinguish them from notes. Edges are wikilinks
and folder links. Click a node to open the note or folder view. Filter by
type, tag, or search query to focus the view. Folder nodes naturally cluster
their children, giving the graph an organic hierarchical structure.

## muninn-cli

The CLI is retained for terminal workflows. It uses `clap` for argument parsing
and calls `muninn-core` directly.

```bash
# Notes
muninn note new "Some Topic" --type til --tags "go,concurrency"
muninn note list --type reference --area work
muninn note search "error handling patterns"
muninn note backlinks some-topic.md

# Search
muninn search "btrfs subvolume permissions"
muninn search "context cancellation" --lang go --type til

# Query
muninn query "SELECT title, status FROM note WHERE status = 'active'"

# Types
muninn type list
muninn type show note

# Validation
muninn validate                    # all notes
muninn validate path/to/note.md   # single note

# Scripting
muninn run scripts/weekly-summary.rhai   # run a script file
muninn render path/to/note.md            # render note with script blocks evaluated

# Export
muninn export path/to/note.md --format pdf
muninn export paper.qmd --format pdf --toc --bibliography refs.bib
muninn export --type design-doc --format html --output ./exports/

# Grammar & spell check
muninn lint                          # all notes
muninn lint path/to/note.md          # single note
muninn lint --type design-doc        # by type

# Citations (Zotero integration)
muninn cite search "smith 2024"                              # search Zotero library
muninn cite export --format bibtex                           # export cited items as .bib
muninn cite export --collection "My Research" --format biblatex
muninn cite key smith2024                                    # look up a citation key

# Maintenance
muninn backfill --type note --field id --dry-run
muninn init
```

Output formats: human-readable table (default), JSON (`--json`), CSV (`--csv`)
for query results.

## Data Location

Unchanged from the Go version:

```
~/.local/share/muninn/
├── .muninn/
│   ├── config.yaml
│   ├── dictionary.txt             # custom words for spell checker
│   ├── types/
│   │   ├── note.md
│   │   ├── journal.md
│   │   └── ...
│   ├── runestones/
│   │   ├── active-work.yaml
│   │   ├── project-tracker.yaml
│   │   └── ...
│   ├── scripts/
│   │   ├── weekly-summary.rhai
│   │   ├── project-dashboard.rhai
│   │   └── ...
│   └── cache/
│       └── index.json
└── notes/
    ├── some-topic.md
    ├── _attachments/              # vault-root attachments
    │   └── logo.png
    ├── projects/
    │   ├── _index.md
    │   ├── _attachments/
    │   │   ├── site-map.png
    │   │   └── inspection-report.pdf
    │   └── plant-ops.md
    ├── journal/
    │   └── 2026-04-W1-05.md
    └── ...
```

The vault path is configurable via `MUNINN_VAULT_PATH` environment variable or
Tauri app settings.

## Dependencies

### Rust (muninn-core)

| Crate | Purpose | Replaces (Go) |
|---|---|---|
| `comrak` | Markdown parsing (CommonMark + GFM) | `goldmark` |
| `serde` + `serde_yaml` | YAML frontmatter deserialization | `gopkg.in/yaml.v3` |
| `serde_json` | JSON serialization for IPC and CLI output | `encoding/json` |
| `yaml-rust2` | Node-level YAML for round-trip preservation | `yaml.v3` Node API |
| `sqlparser` | SQL parsing | `vitess-sqlparser` (Go) |
| `regex` | Wikilink extraction | `regexp` |
| `walkdir` | Recursive directory traversal | `filepath.WalkDir` |
| `notify` | Cross-platform file watching | `fsnotify` |
| `chrono` | Date/time handling | `time` |
| `indexmap` | Ordered maps (field order preservation) | — |
| `tokio` | Async runtime | goroutines |
| `thiserror` | Error types | `errors` / `fmt.Errorf` |
| `uuid` | UUID generation for generated fields | `google/uuid` |
| `slug` | URL-safe slug generation | — |
| `rhai` | Embedded scripting engine | — |
| `harper-core` | Grammar + spell checking engine | — |
| `axum` | HTTP API server | — |
| `tower-http` | CORS, logging middleware | — |
| `reqwest` | HTTP client for Zotero Web API + local server | — |

### Rust (muninn-cli)

| Crate | Purpose | Replaces (Go) |
|---|---|---|
| `clap` | CLI argument parsing | `cobra` |
| `tabled` | Table output formatting | `tabwriter` |
| `colored` | Terminal colors | — |

### Rust (muninn-tauri)

| Crate | Purpose |
|---|---|
| `tauri` v2 | Desktop shell, IPC, window management |
| `tauri-plugin-fs` | File system access from frontend (if needed) |

### Frontend (React)

| Package | Purpose |
|---|---|
| `react` | UI framework |
| `@codemirror/lang-markdown` | Markdown editor |
| `@codemirror/autocomplete` | Wikilink + field completions |
| `@tauri-apps/api` | Tauri command invocation + events |
| `d3-force` or `vis-network` | Graph visualization |
| `react-router` | View navigation |

## Configuration

Two sources:

1. **Vault config** — `.muninn/config.yaml` (mdbase collection settings,
   portable with the vault)
2. **App config** — Tauri's app data directory (window size, theme, editor
   preferences, vault path — local to the machine)

The vault config is the same one the CLI reads. App config is desktop-only and
does not affect CLI behavior.

## Implementation Order

| Phase | Scope | Delivers |
|---|---|---|
| 1 | `muninn-core` — vault, markdown, wikilink (with folder links + attachments), mdbase (types + validation), grammar (harper-core) | Library with full vault operations, type system, folder wikilinks, attachment linking, and grammar checking |
| 2 | `muninn-cli` — clap commands over core | Working CLI, feature parity with Go version + mdbase + folder links |
| 3 | `muninn-core` — query module (SQL parser + evaluator, `Vault::query`, CLI `muninn query`) | Structured queries via CLI; JOINs and `computed:` fields deferred to Phase 5 |
| 4 | `muninn-core` — scripting module (Rhai engine + vault API) | Inline script blocks, CLI `muninn run` / `muninn render` |
| 5 | `muninn-core` — runestones module + API (axum) | Relational views + HTTP access for Asgard tools |
| 6 | `muninn-core` — export module (pandoc + Quarto pipeline) + zotero module (Web API v3 + HTTP citing) | Document export via CLI, `.qmd` as a note type, Zotero citation search + bibliography export |
| 7 | `muninn-tauri` — Tauri shell + React frontend (editor, search, list) | Desktop app MVP with script blocks, attachment embeds, export button, citation picker |
| 8 | `muninn-tauri` — Runestones view, graph view (with folders), query UI, type manager | Full desktop feature set |
| 9 | `muninn-core` — file watching, persistent cache | Live updates, large vault performance |
| 10 | `muninn-core` — migrations, backfill, path patterns | Schema evolution tooling |

Phases 1–2 validate the Rust port with folder wikilinks and attachments built
in from day one. Phase 3 adds the query engine (SELECT, WHERE, GROUP BY,
HAVING, ORDER BY, LIMIT/OFFSET, scalar + aggregate built-ins); JOINs and
`TypeDef.computed` fields ride along with Runestones in Phase 5. Phase 4 adds
scripting on top of the query engine — scripts need `query()` to be useful. Phase 5 builds
Runestones and the API — this is where Muninn becomes useful to the rest of the
Asgard ecosystem, even before the GUI exists. Phase 6 adds export — the
pipeline depends on scripting (to evaluate script blocks before export) and
the vault (to resolve links and attachments). Zotero ships with export because
citations feed into the bibliography pipeline — `[@citekey]` in notes,
`.bib` from Zotero, pandoc renders the references. The HTTP citing protocol
(citation picker) lights up in Phase 7 when the desktop app arrives.
Phases 7–8 build the desktop app. Phases 9–10 round out the feature set.

The API ships before the desktop app. This means Huginn and scripts can start
consuming Muninn data as soon as the core + API are working, without waiting
for the GUI.

The Go codebase is not modified during the rewrite. Both versions can operate
on the same vault simultaneously (both are reading/writing plain files). The
Go version is retired when the Rust version reaches feature parity.

## What Stays the Same

- Notes are plain markdown files on disk
- Git versioning works (everything is text)
- Wikilinks work the same way
- Text search available alongside structured queries
- Same vault layout, same frontmatter format
- No database, no cloud, no external runtime dependencies
- `MUNINN_VAULT_PATH` environment variable
- GPLv3 license

## What Changes

| Before (Go) | After (Rust + Tauri) |
|---|---|
| CLI + LSP interfaces | CLI + desktop app + HTTP API |
| Edit notes in external editor | Built-in markdown editor |
| Wikilink navigation via LSP | Click-to-navigate in app |
| Notes only in link graph | Notes and folders in link graph (`[[folder/]]`) |
| Diagnostics in VS Code | Problems panel in app |
| Completions via LSP protocol | CodeMirror extensions |
| No GUI | Full desktop GUI with Runestones, graph, search, query |
| No structured views | Runestones — relational spreadsheet over typed notes |
| No inline computation | Rhai script blocks — dynamic queries and data in notes |
| Need plugins for custom queries | Built-in: SQL + scripting, no plugins needed |
| No attachment management | `_attachments/` directories with embed/link syntax |
| No export capability | pandoc + Quarto export to PDF, HTML, DOCX, LaTeX, EPUB |
| No computational documents | First-class `.qmd` support with `{python}`, `{r}`, `{julia}` |
| No grammar/spell checking | harper-core built in — real-time checking, custom dictionary |
| No programmatic access | HTTP API for Huginn, Odin, scripts |
| Hardcoded schema | mdbase types from the start |
| Text search only | Text search + SQL queries |
| VS Code extension required | No extension needed |
| Standalone tool | Integrated into Asgard ecosystem via API |
| No citation management | Zotero integration — search library, export `.bib`, citation picker in editor |
| ~6 Go dependencies | Rust crate ecosystem |

## Relationship to Asgard Ecosystem

Both Odin and Muninn are desktop apps with React frontends:

- **Odin** — Wails v2 + React (Go backend). Schema builder, inspection UI.
- **Muninn** — Tauri v2 + React (Rust backend). Knowledge base, note management.
- **Huginn** — AI/analysis. Consumes Muninn data via HTTP API for context.
- **Heimdall** — Go config library. Serves Odin and Huginn. Muninn is
  independent — vault config is self-contained in `.muninn/config.yaml`.

**Integration via API:**

The HTTP API is the integration point. Muninn does not import or depend on
other Asgard tools, but other tools can reach into Muninn:

- **Huginn** queries the vault for context when analyzing data — pull relevant
  notes, search for related knowledge, read structured fields via Runestones
- **Odin** could link inspection records to Muninn notes (e.g. "see design doc
  for this schema")
- **Scripts** create notes from automation (incident reports, daily journals,
  data imports)

This is a one-way dependency: tools consume Muninn, Muninn doesn't know about
them. The API is internal — no versioning guarantees beyond what the team needs,
no auth complexity, localhost only.

A shared React component library between Odin and Muninn is a future
possibility (markdown rendering, frontmatter editors, etc.) but is not a
requirement for the rewrite.
