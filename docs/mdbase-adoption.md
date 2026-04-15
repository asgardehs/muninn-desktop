---
project: Muninn
description:
  Design document for adopting the mdbase specification (v0.2.x) across all
  conformance levels, migrating Muninn from hardcoded schema to user-defined
  types with structured querying. All internals consolidated under .muninn/
  directory to keep the vault root clean.
doc_date: 2026-04-10
doc_rev_date: 2026-04-11
supersedes:
written_by: Adam Bick
---

# mdbase Specification Adoption — Design Document

_Evolving Muninn from text search to structured, typed, queryable markdown._

## Motivation

When Muninn dropped SQLite in favor of flat files (2026-04-06), it gained
simplicity and portability but lost structured querying. The mdbase specification
(https://mdbase.dev/spec.html) restores that capability while staying in the
flat-file world — types become data files, validation becomes schema-driven, and
queries become expressions over frontmatter fields. No database required.

This document covers the full adoption path across all six mdbase conformance
levels, broken into phases that each deliver standalone value.

## Current State

Muninn today:

- Notes are `.md` files in `~/.local/share/muninn/notes/`
- Frontmatter schema is hardcoded in `internal/markdown/schema.go`
- Fixed field vocabulary: title, tags, type, status, area, project, language,
  description, created, updated, supersedes
- Fixed enum values (e.g. type must be design-doc|til|reference|...)
- Search is word-based text matching with weighted scoring
- Wikilink index is in-memory, bidirectional, thread-safe
- Two interfaces: CLI (cobra) and LSP (stdio)

## Target State

Muninn after full mdbase adoption:

- `.muninn/config.yaml` defines collection settings (mdbase-compliant)
- `.muninn/types/` directory holds type definitions as markdown files
- Schema is user-configurable — add fields, types, constraints without code
  changes; editable directly or via VS Code extension settings
- Structured query language over frontmatter fields
- Link validation at the schema level
- Persistent cache for large vaults
- Migration tooling for schema evolution

## Vault Layout Change

```
~/.local/share/muninn/
├── .muninn/                 # all Muninn internals (NEW)
│   ├── config.yaml          # collection settings (mdbase-compliant)
│   ├── types/               # type definitions
│   │   ├── note.md
│   │   ├── journal.md
│   │   ├── reference.md
│   │   └── ...
│   └── cache/               # persistent index (Phase 4)
│       └── index.json
└── notes/
    ├── some-topic.md
    ├── journal/
    │   └── 2026-04-W1-05.md
    └── ...
```

All configuration, type definitions, and cache live inside `.muninn/`. The
user's note space stays clean — the `notes/` directory contains only their
content. This follows the same convention as Obsidian's `.obsidian/` folder:
internals are accessible but not in the way.

Type definitions in `.muninn/types/` are user-editable files. Users can modify
them directly in any editor, or manage them through the VS Code extension's
settings UI (see Phase 1.9).

---

## Phase 1 — Foundation (Conformance Level 1–2)

_Type definitions as data, not code._

This phase replaces the hardcoded schema with mdbase-compliant type definitions
and adds structured field validation. It is the largest single phase because it
touches every layer of the stack.

### 1.1 — Collection Configuration

Add a new `internal/mdbase/` package to parse and manage the collection config.

**File:** `.muninn/config.yaml` at vault root.

```yaml
spec_version: "0.2.0"
name: "muninn-vault"
description: "Personal knowledge base"
settings:
  types_folder: "types"
  default_validation: "warn"
  default_strict: false
  id_field: "id"
  write_nulls: "omit"
  write_defaults: true
  exclude:
    - ".git"
```

The mdbase spec places `mdbase.yaml` at collection root. Muninn consolidates
all internals under `.muninn/` to keep the vault root clean — the same
convention as Obsidian's `.obsidian/` folder. The config content is
mdbase-compliant; only the file location differs. `types_folder` is relative
to `.muninn/` (defaults to `"types"`).

**Implementation steps:**

1. Create `internal/mdbase/config.go` — struct for config with defaults
2. Add `LoadConfig(vaultRoot string) (*Config, error)` — reads
   `.muninn/config.yaml`, validates, applies defaults for missing settings
3. Update `internal/vault/vault.go` — `New()` loads config if `.muninn/`
   exists; vault operates without config for backwards compatibility
4. Update `cmd/muninn/cmd_init.go` — `muninn init` creates `.muninn/`
   directory with `config.yaml` and `types/` alongside `notes/`
5. Add `.muninn/` to default exclude list so type definition files are not
   indexed as notes

**Backwards compatibility:** If no `.muninn/` directory exists, Muninn falls
back to current behavior (hardcoded schema). This allows gradual migration.

### 1.2 — Type Definition Files

Type definitions are markdown files in `.muninn/types/` with schema frontmatter
and optional documentation body.

**Example** `.muninn/types/note.md`:

```markdown
---
name: note
description: General-purpose note
fields:
  title:
    type: string
    required: true
  tags:
    type: list
    items:
      type: string
    default: []
  status:
    type: enum
    values: [draft, active, complete, archived]
    default: draft
  area:
    type: enum
    values: [personal, work, journal]
  project:
    type: string
  language:
    type: string
  description:
    type: string
  created:
    type: date
  updated:
    type: date
  supersedes:
    type: link
---

# Note

General-purpose knowledge base entry. Use for TILs, references, design
documents, and anything that doesn't fit a more specific type.
```

**Implementation steps:**

1. Create `internal/mdbase/types.go` — structs for type definitions:
   `TypeDef`, `FieldDef`, `MatchRule`
2. Create `internal/mdbase/loader.go` — `LoadTypes(typesDir string)
   (map[string]*TypeDef, error)` — reads all `.md` files in `.muninn/types/`,
   parses frontmatter into type definitions, validates names and structure
3. Implement type name validation: lowercase letters, numbers, hyphens,
   underscores; starts with letter; max 64 chars; name must match filename
4. Create default type definitions that mirror the current hardcoded schema
   (note, journal, reference, design-doc, til, decision, troubleshooting) so
   `muninn init` produces a vault that behaves identically to today

### 1.3 — Field Type System

Implement validation for all mdbase primitive and composite field types.

**Primitives to implement:**

| Type       | Constraints                          |
| ---------- | ------------------------------------ |
| `string`   | `min_length`, `max_length`, `pattern` |
| `integer`  | `min`, `max`                         |
| `number`   | `min`, `max`                         |
| `boolean`  | (none)                               |
| `date`     | ISO 8601 YYYY-MM-DD                  |
| `datetime` | ISO 8601 with optional timezone      |
| `time`     | HH:MM:SS                             |
| `enum`     | `values` array                       |
| `any`      | (none — accepts any YAML value)      |

**Composites:**

| Type     | Constraints                              |
| -------- | ---------------------------------------- |
| `list`   | `items` (element schema), `min_items`, `max_items` |
| `object` | `fields` (nested field schemas)          |
| `link`   | `target` (type name), `validate_exists`  |

**Common field options (all types):**

- `required` — must be present and non-null
- `default` — value applied when field is missing
- `description` — documentation
- `deprecated` — marks field as deprecated
- `unique` — value must be unique across files of this type

**Implementation steps:**

1. Create `internal/mdbase/validate.go` — `ValidateField(value any, field
   *FieldDef) []ValidationError` for each type
2. Create `internal/mdbase/validate_test.go` — comprehensive tests for each
   type and constraint combination
3. Implement null semantics: `null`, `~`, and bare `:` are all null; distinct
   from empty string `""`; `required: true` rejects null
4. Implement default application: missing fields get defaults before validation
5. Implement strict mode: unknown fields rejected when type `strict: true`,
   warned when `strict: "warn"`, ignored when `strict: false`

### 1.4 — Type Matching

Determine which type(s) apply to a given note file.

**Matching precedence (per spec):**

1. Explicit `type:` or `types:` field in frontmatter (highest priority)
2. Match rules defined in type definitions
3. Untyped (no match)

**Match rule conditions (combined with AND):**

- `path_glob` — glob pattern against file path relative to vault root
- `fields_present` — all listed fields must exist and be non-null
- `where` — structured operators: `eq`, `neq`, `gt`, `gte`, `lt`, `lte`,
  `exists`, `contains`, `containsAll`, `containsAny`, `startsWith`,
  `endsWith`, `matches`

**Implementation steps:**

1. Create `internal/mdbase/match.go` — `MatchTypes(filePath string,
   frontmatter map[string]any, types map[string]*TypeDef) []*TypeDef`
2. Handle multi-type files: when a file matches multiple types, it must
   satisfy all schemas; field constraints merge to most restrictive
   intersection
3. Handle `explicit_type_keys` config setting (default: `["type", "types"]`)
4. Silent non-match on missing/null fields or type mismatches in `where`
   conditions (per spec)

### 1.5 — Type Inheritance

Single inheritance via `extends` field in type definitions.

```yaml
---
name: design-doc
extends: note
fields:
  decision:
    type: string
  rationale:
    type: string
---
```

**Rules:**

- Child completely overrides parent fields with same name (no constraint
  merging)
- Strictness inherited unless overridden
- Circular inheritance rejected at load time
- Missing parent type is a load-time error

**Implementation steps:**

1. Add inheritance resolution to `LoadTypes()` — topological sort of type
   graph, detect cycles
2. Merge fields: parent fields first, child fields override by name
3. Merge computed fields: same override behavior

### 1.6 — Generated Fields

Auto-populated fields using the `generated` property.

**Strategies to implement:**

| Strategy       | Behavior                                    |
| -------------- | ------------------------------------------- |
| `now`          | Current datetime on every read              |
| `now_on_write` | Current datetime at write time only         |
| `uuid`         | UUID v4, generated once on creation         |
| `uuid_short`   | Short 8-char UUID, generated once           |
| `slug`         | URL-safe slug derived from specified field   |
| `counter`      | Auto-incrementing number per type           |

**Constraints:** Cannot be `required`, have `default`, or be `computed`.

**Implementation steps:**

1. Create `internal/mdbase/generate.go` — `ApplyGenerated(fields
   map[string]*FieldDef, frontmatter map[string]any, isNew bool)` — applies
   generated values based on strategy and whether the file is new or being
   updated
2. For `counter`, need to scan existing files of the type to determine next
   value
3. For `slug`, implement slugification: lowercase, replace special chars with
   hyphens, transliterate non-ASCII, trim leading/trailing hyphens

### 1.7 — Update Vault Package

Integrate the new type system into the existing vault operations.

**Changes to `internal/vault/`:**

1. `vault.go` — `New()` loads mdbase config and types; expose `Config()` and
   `Types()` accessors
2. `vault.go` — `CreateNote()` accepts type name, validates against type
   schema, applies defaults and generated fields, renders frontmatter from
   type definition instead of hardcoded template
3. `vault.go` — `ReadNote()` returns parsed frontmatter with defaults applied
   and computed fields evaluated (Phase 2 for computed)
4. `search.go` — search continues to work as-is (text-based); Phase 2 adds
   structured queries alongside it
5. `list.go` — `ListFiltered()` uses type matching instead of hardcoded
   field checks
6. `tags.go` — unchanged (tags are still extracted from frontmatter)

**Migration of `internal/markdown/schema.go`:**

The hardcoded `Schema`, `ValidateFrontmatter()`, `ValidateVocabulary()`,
`RenderFrontmatter()`, and `RenderTemplate()` functions are replaced by the
mdbase type system. `schema.go` is removed once the migration is complete.
`frontmatter.go` and `parser.go` remain — they handle YAML parsing and markdown
splitting, which are independent of the schema.

### 1.8 — Update CLI

**Changes to `cmd/muninn/`:**

1. `cmd_init.go` — generate `.muninn/` directory with `config.yaml` and
   default `types/` with type definitions mirroring current schema
2. `cmd_note.go` (new) — `--type` flag now refers to mdbase type names;
   available types are discovered from `.muninn/types/`, not hardcoded
3. `cmd_note.go` (list) — filtering uses type matching; filter flags are
   dynamic based on available type fields
4. `cmd_note.go` (search) — unchanged in this phase (text search still works)
5. Add `muninn type list` command — lists available types with descriptions
6. Add `muninn type show <name>` command — displays type schema and field
   details
7. Add `muninn validate [path]` command — validates a note or all notes
   against their matched types; reports errors and warnings

### 1.9 — Update LSP and VS Code Extension

**Changes to `internal/lsp/`:**

1. `server.go` — load mdbase config and types on initialization; store on
   server struct; watch `.muninn/types/` for changes and hot-reload type
   definitions when files are added, modified, or deleted
2. `diagnostics.go` — replace hardcoded frontmatter validation with mdbase
   type validation; report `missing_required`, `type_mismatch`,
   `constraint_violation`, `unknown_type` as LSP diagnostics
3. `completion.go` — enum completions driven by type field `values` instead of
   hardcoded vocabulary; field name completions from matched type schema
4. `commands.go` — `muninn/createNote` uses type definitions for field
   generation and defaults
5. `hover.go` — optionally show type information in hover for frontmatter
   fields
6. Add LSP commands for type management:
   - `muninn/listTypes` — returns all loaded type definitions with fields
   - `muninn/getType` — returns full schema for a single type
   - `muninn/createType` — creates a new type definition file
   - `muninn/updateType` — updates fields/constraints on an existing type

**Changes to `vscode/`:**

The VS Code extension provides a settings UI for managing types without editing
`.muninn/types/` files directly. This mirrors how Obsidian exposes plugin
settings through its UI while storing them as files in `.obsidian/`.

1. Add "Muninn: Types" view in the sidebar — tree view listing all types
   with their fields, constraints, and inheritance
2. Add "Muninn: Create Type" command — form-based type creation that writes
   to `.muninn/types/`
3. Add "Muninn: Edit Type" command — opens a webview form for modifying
   type fields, adding/removing constraints, and setting defaults
4. Type changes made through the UI write directly to `.muninn/types/*.md`
   files — the extension is just a convenience layer over the same files
5. Changes to type files (whether from the UI or direct editing) trigger
   LSP reload via file watch, so diagnostics and completions update
   immediately

### 1.10 — Phase 1 Testing

- Unit tests for config loading, type parsing, field validation, type
  matching, inheritance resolution, generated fields
- Integration tests: create vault with `mdbase.yaml` and types, create notes,
  validate, list, search
- Migration test: vault without `mdbase.yaml` continues to work (backwards
  compat)
- LSP tests: diagnostics, completions, and code actions with type-driven
  schema

---

## Phase 2 — Query Language (Conformance Level 3)

_SQL syntax over frontmatter fields._

This phase adds a SQL query language as the primary way to find and filter
notes by field values. The mdbase spec defines the required query *semantics*
(filtering, sorting, pagination, link traversal) but leaves the concrete
syntax to the implementation. SQL is a natural fit — Muninn lost SQL when it
dropped SQLite, and this brings back the same mental model without a database.

Text search remains available for body content matching via `muninn search`.

### 2.1 — SQL Dialect Design

Muninn SQL is a read-only subset of SQL operating on frontmatter fields.
Types are tables, fields are columns, notes are rows.

**Supported clauses:**

```sql
SELECT title, status, tags
FROM note
WHERE status != 'archived' AND tags CONTAINS 'go'
ORDER BY updated DESC
LIMIT 10
```

**Clause mapping to mdbase concepts:**

| SQL Clause   | mdbase Concept       | Notes                              |
| ------------ | -------------------- | ---------------------------------- |
| `SELECT`     | field projection     | `*` for all fields                 |
| `FROM`       | type filter          | type name(s), aliases supported    |
| `WHERE`      | expression filter    | comparisons, boolean logic, functions |
| `JOIN`       | link traversal       | resolves link fields to target notes |
| `GROUP BY`   | aggregation          | extension beyond spec              |
| `HAVING`     | post-aggregation filter | extension beyond spec           |
| `ORDER BY`   | sorting              | ASC/DESC per field                 |
| `LIMIT`      | pagination           | with optional `OFFSET`             |

**WHERE operators:**

| Operator                   | Example                                  |
| -------------------------- | ---------------------------------------- |
| `=`, `!=`, `<`, `>`, `<=`, `>=` | `status = 'active'`                |
| `AND`, `OR`, `NOT`         | `status = 'active' AND area = 'work'`    |
| `LIKE`                     | `title LIKE '%error%'`                   |
| `IN`                       | `status IN ('draft', 'active')`          |
| `IS NULL`, `IS NOT NULL`   | `project IS NOT NULL`                    |
| `CONTAINS`                 | `tags CONTAINS 'go'` (list membership)   |
| `CONTAINS ALL`             | `tags CONTAINS ALL ('go', 'concurrency')` |
| `CONTAINS ANY`             | `tags CONTAINS ANY ('go', 'rust')`       |
| `BETWEEN`                  | `created BETWEEN '2026-01-01' AND '2026-04-01'` |

**Built-in functions:**

| Function        | Returns   | Description                         |
| --------------- | --------- | ----------------------------------- |
| `TODAY()`       | date      | Current date in configured timezone |
| `NOW()`         | datetime  | Current datetime with timezone      |
| `COUNT(*)`      | integer   | Row count (for GROUP BY)            |
| `LENGTH(field)` | integer   | String length or list length        |
| `LOWER(field)`  | string    | Lowercase string                    |
| `UPPER(field)`  | string    | Uppercase string                    |
| `COALESCE(a,b)` | any       | First non-null value                |
| `DATE_ADD(d,interval)` | date | Date arithmetic (`DATE_ADD(created, '7d')`) |
| `EXISTS(field)` | boolean   | Whether field key is present        |

**File metadata (accessible as `file.*` columns):**

`file.path`, `file.basename`, `file.name`, `file.body`, `file.size`,
`file.created`, `file.modified`, `file.types`

**JOIN for link traversal:**

```sql
SELECT n.title, s.title AS superseded_title
FROM note n
JOIN note s ON n.supersedes = s.id
WHERE n.status = 'active'
```

Link fields resolve to the target note's frontmatter when joined.

### 2.2 — SQL Parser

Leverage an existing Go SQL parser library rather than building from scratch.
This significantly reduces implementation effort.

**Candidate libraries:**

- `github.com/xwb1989/sqlparser` — Vitess-derived, mature, well-tested
- `github.com/pingcap/tidb/parser` — full MySQL parser, heavier but complete
- `github.com/blastrain/vitess-sqlparser` — lighter Vitess fork

**Implementation steps:**

1. Evaluate parser libraries — pick one that handles the subset we need
   without pulling in excessive dependencies
2. Create `internal/mdbase/query/` package
3. `parser.go` — wraps the chosen library, converts SQL AST into Muninn's
   internal query representation; rejects unsupported syntax (DDL, subqueries,
   INSERT/UPDATE/DELETE) with clear error messages
4. `ast.go` — internal query AST: `Query` struct with Select, From, Where,
   Join, GroupBy, Having, OrderBy, Limit fields
5. `parser_test.go` — test cases for all supported clause combinations and
   rejection of unsupported syntax

### 2.3 — Query Evaluator

Execute parsed queries against the vault.

**Implementation steps:**

1. `eval.go` — query executor:
   - Resolve `FROM` clause to type(s) and discover matching files
   - Resolve `JOIN` clauses by following link fields
   - Evaluate `WHERE` clause against each note's frontmatter
   - Apply `GROUP BY` and `HAVING` if present
   - Sort by `ORDER BY` fields
   - Apply `LIMIT` and `OFFSET`
   - Project `SELECT` fields
   - Return `[]QueryResult` with selected fields and file metadata
2. `eval_context.go` — holds vault reference, type info, timezone, and
   provides field resolution for a given note
3. `functions.go` — built-in SQL function registry (TODAY, NOW, COUNT,
   LENGTH, LOWER, UPPER, COALESCE, DATE_ADD, EXISTS)
4. Resource limits: max result set size, evaluation timeout
5. `eval_test.go` — tests for each clause, function, and operator

### 2.4 — Computed Fields

Read-only fields derived from SQL expressions, evaluated at read time.

```yaml
fields:
  full_name:
    type: string
    computed: "CONCAT(first_name, ' ', last_name)"
```

**Rules:**

- Evaluated against effective frontmatter (defaults applied, other computed
  fields excluded from input)
- Not persisted to disk
- Available in queries as regular columns
- Circular dependencies rejected at type load time
- Persisted frontmatter values with matching field name are ignored

**Implementation steps:**

1. Add computed field support to `internal/mdbase/types.go`
2. Dependency analysis at type load time — topological sort of computed fields,
   reject cycles
3. Evaluation during read: after defaults applied, before query filtering
4. Tests for computed fields, circular detection, override behavior

### 2.5 — CLI Query Command

```bash
# SQL query
muninn query "SELECT title, status FROM note WHERE status = 'active' ORDER BY updated DESC"

# Text search still available
muninn search "error handling"

# Combined: text search with type filter
muninn search "error handling" --type til
```

**Implementation steps:**

1. Add `cmd_query.go` — `muninn query "<sql>"` takes a SQL string as argument
2. Output formats: table (default), JSON (`--json`), CSV (`--csv`)
3. Keep `muninn search` as text search — the two complement each other
4. Interactive mode (future): `muninn query` with no args opens a REPL

### 2.6 — LSP Query Integration

- Add command `muninn/query` executable from VS Code command palette
- Accepts SQL string, returns results as locations for navigation
- Workspace symbol search can optionally use SQL WHERE clauses

---

## Phase 3 — Links & References (Conformance Level 4–5)

_Schema-level link validation and reference management._

Muninn already has strong wikilink support. This phase formalizes it within the
mdbase type system and adds link-level validation.

### 3.1 — Link Field Type

The `link` field type in type definitions adds schema-level link semantics.

```yaml
fields:
  supersedes:
    type: link
    target: note         # must link to a note-type file
    validate_exists: true
  related:
    type: list
    items:
      type: link
      target: note
```

**Implementation steps:**

1. Add link resolution to field validation — when `validate_exists: true`,
   check that the linked file exists in the vault
2. Support resolution strategies: explicit path, wikilink-style name lookup,
   `id_field` matching
3. Report `missing_link_target` as validation error/warning
4. Support multi-type targets: `target: [note, reference]`

### 3.2 — Link Traversal in Expressions

Enable `assignee.asFile().team` syntax in query expressions.

**Implementation steps:**

1. Add `.asFile()` method to link values in the evaluator — resolves to the
   linked file's frontmatter
2. Allow chained field access on resolved files
3. Handle missing targets gracefully (return undefined)

### 3.3 — Markdown Link Support

Currently Muninn only extracts wikilinks. mdbase also supports standard markdown
links as references: `[text](path.md)`.

**Implementation steps:**

1. Add markdown link extraction to `internal/wikilink/extract.go` (or a new
   `internal/links/` package)
2. Include markdown links in the bidirectional index
3. Both syntaxes resolve through the same pipeline

### 3.4 — Reference Updates on Rename (Level 5)

Muninn's LSP already handles rename with cascading wikilink updates. This phase
ensures it also covers:

1. Markdown-style links (`[text](path.md)`)
2. Bare path references in frontmatter link fields
3. Respects `settings.rename_update_refs` config flag

### 3.5 — Backlinks in Expressions

Add `file.backlinks()` function for query expressions.

```
file.backlinks().length > 5
```

**Implementation steps:**

1. Expose backlink data through the expression evaluator's file metadata
2. Returns list of file objects for chained access

---

## Phase 4 — Caching & Performance (Conformance Level 5 continued)

_Persistent index and file watching for large vaults._

### 4.1 — Persistent Cache

Currently the wikilink index is rebuilt from scratch on every LSP server start.
For large vaults this becomes slow.

**Cache location:** `.muninn/cache/` directory.

**What to cache:**

- Parsed frontmatter per file (keyed by path + modification time)
- Wikilink index (forward links and backlinks)
- Type matching results per file
- Tag collection

**Implementation steps:**

1. Create `internal/mdbase/cache.go` — JSON-based cache with file modification
   time invalidation
2. On startup: load cache, check modification times, incrementally update
   stale entries
3. On file change: update affected cache entries
4. Cache is advisory — always correct to delete `.muninn/cache/` and rebuild

### 4.2 — File Watching

When Muninn runs as a persistent process (LSP server or future HTTP server),
watch the vault directory for external changes.

**Implementation steps:**

1. Add `fsnotify` dependency for cross-platform file watching
2. Watch vault directory recursively for create, modify, delete, rename events
3. On change: reparse affected file, update wikilink index, update cache,
   republish LSP diagnostics
4. Debounce rapid changes (e.g. git checkout touching many files)
5. Handle `.muninn/` changes: reload config and types when type definitions
   or config are modified

### 4.3 — Query Performance

With the cache in place, queries can read from cached frontmatter instead of
parsing every file on every query.

**Implementation steps:**

1. Query executor checks cache first, falls back to file read
2. Index commonly queried fields for faster filtering (optional)
3. Benchmark with 1k, 5k, 10k note vaults to establish performance baseline

---

## Phase 5 — Migrations & Advanced Features (Conformance Level 6)

_Schema evolution and bulk operations._

### 5.1 — Backfill Command

Apply defaults or generated values to existing files that predate a schema
change.

```bash
muninn backfill --type note --field status --dry-run
muninn backfill --type note --field id
```

**Implementation steps:**

1. Add `cmd_backfill.go` — scans files of given type, applies missing defaults
   or generated values
2. `--dry-run` flag shows what would change without writing
3. Preserves existing frontmatter formatting (only adds/updates target fields)
4. Reports count of files modified

### 5.2 — Migration Manifests

Structured schema evolution steps for renaming fields, changing types, or
splitting/merging types.

```yaml
# .muninn/migrations/001-rename-lang.yaml
version: 1
steps:
  - action: rename_field
    type: note
    from: language
    to: lang

  - action: add_field
    type: note
    field: priority
    default: "medium"
```

**Implementation steps:**

1. Create `internal/mdbase/migrate.go` — migration manifest parser and
   executor
2. Support actions: `rename_field`, `add_field`, `remove_field`,
   `change_type`, `rename_type`, `move_files`
3. Migration state tracking: record which migrations have been applied
4. `muninn migrate` command — applies pending migrations
5. `muninn migrate --dry-run` — shows planned changes
6. Rollback support (best-effort — some actions are inherently one-way)

### 5.3 — Unique Field Enforcement

Cross-file uniqueness constraints.

**Implementation steps:**

1. During validation, scan all files of the type and check for duplicate values
2. Report `duplicate_value` with paths of conflicting files
3. Cache unique value sets for performance
4. Null/missing values exempt from uniqueness checks

### 5.4 — Path Patterns

Type-level path conventions with template variables.

```yaml
---
name: journal
path_pattern: "journal/{date}.md"
---
```

**Implementation steps:**

1. Add path pattern resolution to type definitions
2. Variables: `{id}`, `{slug}`, `{date}`, `{title}` with auto-slugification
3. Validate file paths against expected patterns (warning-level by default)
4. Use patterns in `muninn note new` to auto-place files in the right
   directory

---

## Cross-Cutting Concerns

### Backwards Compatibility

- Vaults without `.muninn/` continue to work with current behavior
- `muninn init` on an existing vault adds `.muninn/` with config and types
  without modifying existing notes
- Existing notes remain valid — the default types mirror the current hardcoded
  schema
- `muninn migrate` provides a path from old to new without manual editing

### Error Handling

mdbase defines specific error codes. Map these to user-facing messages:

| Code                    | Severity | Message Pattern                             |
| ----------------------- | -------- | ------------------------------------------- |
| `missing_required`      | error    | "Field '{field}' is required in type '{type}'" |
| `type_mismatch`         | error    | "Field '{field}' expected {expected}, got {actual}" |
| `constraint_violation`  | warning  | "Field '{field}' violates {constraint}"     |
| `unknown_type`          | warning  | "Type '{type}' is not defined"              |
| `type_conflict`         | error    | "Field '{field}' conflicts between types"   |
| `duplicate_value`       | error    | "Unique field '{field}' duplicated in {paths}" |
| `invalid_type_definition` | error  | "Type '{type}' definition is invalid: {reason}" |

### Frontmatter Preservation

The mdbase spec requires round-trip preservation of untouched fields. Muninn
must not reorder, re-quote, or reformat fields it doesn't modify. This means:

- Use `gopkg.in/yaml.v3` node-level API for surgical edits (not
  marshal/unmarshal round-trips)
- Preserve comments, blank lines, and string quoting style
- Only normalize indentation (2 spaces) and trailing whitespace

### Security

- Regex patterns in field constraints and expressions: enforce ES2018+ subset,
  guard against ReDoS with evaluation timeouts
- Expression evaluation: set nesting depth limit, function call count limit,
  and wall-clock timeout
- No shell execution or file system access from expressions

### Testing Strategy

Each phase includes its own test suite. Additionally:

- **Conformance tests**: once available from the mdbase project, run the
  official conformance test suite to verify compliance at each level
- **Vault fixtures**: maintain test vaults in `testdata/` with representative
  note collections for integration testing
- **Benchmark suite**: measure query performance at various vault sizes
  (100, 1k, 10k notes)

---

## Implementation Order

| Phase | Level | Effort   | Depends On | Delivers                          |
| ----- | ----- | -------- | ---------- | --------------------------------- |
| 1     | 1–2   | Large    | —          | Dynamic types, validation, CLI    |
| 2     | 3     | Medium   | Phase 1    | SQL query language, computed fields |
| 3     | 4–5   | Medium   | Phase 1    | Link validation, ref updates      |
| 4     | 5     | Medium   | Phase 1    | Cache, file watching, performance |
| 5     | 6     | Medium   | Phase 1–2  | Migrations, backfill, path patterns |

Phase 2 effort reduced from Large to Medium by using an existing Go SQL parser
library instead of building a custom expression language from scratch. Phases
2, 3, and 4 can be worked in parallel after Phase 1 is complete. Phase 5
depends on both Phase 1 (types) and Phase 2 (SQL evaluator for computed fields
in migrations).

---

## What Stays the Same

- Notes are still plain markdown files on disk
- Git versioning still works (everything is text)
- Wikilinks still work the same way
- Text search still available alongside structured queries
- LSP server still communicates over stdio
- CLI still uses cobra
- No database, no cloud, no external runtime dependencies

## What This Enables Later

- **HTTP API / server mode**: the type system and query language map directly
  to API endpoints; frontends can discover schema from the server
- **Web UI**: structured types enable form-based note creation; queries enable
  filtered views and dashboards
- **Multi-device access**: server serves the vault; any client connects
- **Cross-vault queries**: mdbase collections are self-describing; tooling can
  query across multiple vaults
