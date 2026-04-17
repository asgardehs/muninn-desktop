---
layout: default
title: Runestones
permalink: /docs/muninn/runestones/
---

# Runestones

A **Runestone** is a saved, named view of your vault — a spreadsheet
over typed notes. You pick a type, choose which fields become columns,
add a filter and a sort, and store the whole thing as YAML. The rows
are always the live notes on disk; a Runestone is a lens, not a copy.

Think of them as the bridge between
[queries](/docs/muninn/query/) (one-shot SQL) and the desktop UI
(interactive tables). The CLI can evaluate a Runestone the same way
the Tauri app will — which makes them portable across the CLI, the
HTTP API, and the eventual editor.

## Where Runestones live

```
.muninn/
└── runestones/
    ├── active-work.yaml
    └── weekly-journal.yaml
```

Any `.yaml` or `.yml` file in `.muninn/runestones/` is a Runestone.
The file stem is a convenient shortcut for `muninn runestone` commands,
but the authoritative name is the `name:` field inside the file.

## File format

```yaml
name: Active Work
description: All active tasks, highest priority first
source:
  types: [task]
  filter: "status = 'active'"
columns:
  - field: title
    width: 200
  - field: priority
    sort: desc
  - field: project
  - field: days_open
    computed: "DATE_ADD(TODAY(), 0) - DATE_ADD(created, 0)"
order_by:
  - field: priority
    sort: desc
limit: 100
```

### Top-level fields

| Field         | Type              | Purpose                                                         |
| ------------- | ----------------- | --------------------------------------------------------------- |
| `name`        | string            | Display name; how the Runestone is addressed                    |
| `description` | string            | One-line summary, shown by `muninn runestone list`              |
| `source`      | map               | Where rows come from (see below)                                |
| `columns`     | list of column defs | Which columns appear and how they behave                      |
| `order_by`    | list              | Runestone-level sort; wins over per-column `sort:` hints        |
| `limit`       | integer           | Cap the number of rows                                          |
| `group_by`    | string            | Presentational grouping column (UI header rows, not SQL GROUP BY) |

### `source`

| Field    | Purpose                                                |
| -------- | ------------------------------------------------------ |
| `types`  | List of source types. This version supports exactly one. |
| `filter` | SQL `WHERE` clause body (without the `WHERE` keyword). |

Multi-type Runestones (UNION-style) are deferred — use a separate
Runestone per type for now.

### `columns`

Each column is either a direct frontmatter field or a virtual column
with an inline SQL expression.

| Field      | Purpose                                                            |
| ---------- | ------------------------------------------------------------------ |
| `field`    | Column identifier. For regular columns this is a frontmatter field or a [computed field](/docs/muninn/types/#computed-fields) on the type. For virtual columns, this is the name the cell is addressed by. |
| `header`   | Display header. Defaults to `field`.                               |
| `width`    | Pixel width hint for the UI. Ignored by the CLI.                   |
| `sort`     | `asc` / `desc`. Used only when `order_by:` isn't set at the top level. |
| `hidden`   | Omit the column from the rendered output.                          |
| `computed` | Inline SQL expression. The column doesn't need to exist in frontmatter. |

Virtual computed columns are local to the Runestone — they don't show
up in plain `muninn query` calls against the underlying type. For
columns you want available everywhere, put them in the type's
[`computed:` map](/docs/muninn/types/#computed-fields) instead.

## CLI

```bash
muninn runestone list
muninn runestone show <name>
muninn runestone eval <name> [--with-path] [--json]
```

`name` matches against the `name:` field first, then falls back to the
YAML filename stem. Pass `--json` to get structured output.

```bash
muninn runestone eval "Active Work" --with-path --json
```

## How it evaluates

Evaluation compiles the Runestone to SQL and runs it through the
[query engine](/docs/muninn/query/). Roughly:

```sql
SELECT title, priority, project, (DATE_ADD(TODAY(), 0) - DATE_ADD(created, 0)) AS days_open
FROM task
WHERE status = 'active'
ORDER BY priority DESC
LIMIT 100
```

That means everything the query engine supports is available in the
filter and in computed expressions: scalar functions, arithmetic,
`LIKE`, `IS NULL`, etc. If a filter has a syntax error, you'll see the
usual query parse error — just at `runestone eval` time rather than
YAML-load time.

## Writeback

Runestones are editable — the desktop app will surface inline cell
edits, and the HTTP API exposes the same operation at
`PUT /api/runestones/<name>/rows/<path>`. A cell edit rewrites the one
frontmatter field on the owning note and leaves everything else
(including the markdown body) untouched.

Computed columns — whether on the type or inline on the Runestone —
are read-only.

## When to reach for Runestones

- You find yourself re-typing the same `muninn query` with the same
  filter and sort combo
- You want a structured table over typed notes that the editor can
  turn into a spreadsheet later
- You need to share a canonical view with other Asgard tools via the
  HTTP API

For one-off questions, use [`muninn query`](/docs/muninn/query/)
directly. For "find me a note about X", use
[Search](/docs/muninn/search/).

See also: [Types](/docs/muninn/types/) for declaring the fields
Runestones lean on, and [Query](/docs/muninn/query/) for the
expression syntax that shows up in filters and computed columns.
