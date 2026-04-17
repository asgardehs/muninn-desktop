---
layout: default
title: HTTP API
permalink: /docs/muninn/http-api/
---

# HTTP API

`muninn-server` hosts the same vault operations the CLI runs, exposed
as a JSON HTTP API. It's how Huginn, Odin, automation scripts, and
(eventually) the Tauri desktop shell talk to a vault — one vault, one
process, shared across tools.

The API is **internal to the Asgard ecosystem**. There's no auth, no
rate limiting, no backward-compatibility promise between versions.
Don't expose it to the public internet; the default bind is
`127.0.0.1` for a reason.

## Starting the server

```bash
muninn-server --vault ~/.local/share/muninn
```

```
muninn-server listening on http://127.0.0.1:9200
```

### Flags

| Flag              | Default         | Meaning                                                      |
| ----------------- | --------------- | ------------------------------------------------------------ |
| `--vault <path>`  | —               | Vault root. Also `MUNINN_VAULT_PATH` env var.                |
| `--bind <addr>`   | `127.0.0.1`     | Interface to listen on.                                      |
| `--port <n>`      | `9200`          | Preferred port. See port fallback below.                     |
| `--strict-port`   | off             | Refuse to start if `--port` is unavailable.                  |

### Port fallback

If the preferred port is taken, the server scans `port+1` through
`port+99` and binds the first one that works. The actual bound
address is printed to stdout once on startup — scripts that need to
know which port the server chose can parse that line:

```
muninn-server listening on http://127.0.0.1:9201
```

Pass `--strict-port` to skip the scan and fail loudly instead.

### Shutdown

`muninn-server` shuts down gracefully on `SIGINT` (Ctrl-C) and
`SIGTERM`. Currently in-flight requests complete before the listener
stops.

## Endpoints

All endpoints live under `/api`. Request and response bodies are JSON.
Errors come back as `{ "error": "message", "code": "<kind>" }` with
an HTTP status:

| Status | `code`        | Meaning                                      |
| ------ | ------------- | -------------------------------------------- |
| 400    | `bad_request` | Malformed input or semantic error            |
| 404    | `not_found`   | No such note / type / runestone              |
| 409    | `conflict`    | Resource conflict (reserved; unused so far)  |
| 500    | `internal`    | Unexpected server error                      |

### Notes

| Method | Path                    | Purpose                                           |
| ------ | ----------------------- | ------------------------------------------------- |
| `GET`  | `/api/notes`            | List notes with optional `?type=`, `?tag=`, `?title=` |
| `POST` | `/api/notes`            | Create a note (body: `{title, type?, fields?}`)   |
| `GET`  | `/api/notes/*path`      | Read a note as `{path, title, frontmatter, body, tags}` |
| `PUT`  | `/api/notes/*path`      | Full-replace the note (body: `{frontmatter, body}`) |
| `DELETE` | `/api/notes/*path`    | Delete the note                                   |

`PUT` is a full replace, not a patch — whatever you send is the new
frontmatter. Read first, modify, send back.

### Search

```
GET /api/search?q=<query>&type=<name>&limit=<n>
```

Returns scored matches with snippets:

```json
{
  "query": "osha standards",
  "results": [
    { "path": "references/osha-standards.md", "title": "OSHA Standards Reference", "score": 9, "snippet": "..." }
  ]
}
```

### Query

```
POST /api/query
{ "sql": "SELECT title, status FROM task WHERE status = 'active'" }
```

```json
{
  "columns": ["title", "status"],
  "rows": [
    { "path": "task-a.md", "cells": { "title": "Task A", "status": "active" } }
  ]
}
```

Same engine as `muninn query` — see [Query](/docs/muninn/query/) for
the full SQL subset.

### Runestones

| Method | Path                                        | Purpose                                    |
| ------ | ------------------------------------------- | ------------------------------------------ |
| `GET`  | `/api/runestones`                           | List saved Runestones (summary only)       |
| `GET`  | `/api/runestones/<name>`                    | Evaluate the Runestone and return rows     |
| `PUT`  | `/api/runestones/<name>/rows/<path>`        | Update one cell (body: `{column, value}`)  |

See [Runestones](/docs/muninn/runestones/) for the YAML format.

Cell writeback rejects computed/virtual columns with a `bad_request`.

### Types

| Method | Path                  | Purpose                                                |
| ------ | --------------------- | ------------------------------------------------------ |
| `GET`  | `/api/types`          | List type names                                        |
| `GET`  | `/api/types/<name>`   | Full type definition (fields, computed map, inheritance) |

### Links

| Method | Path                              | Purpose                                           |
| ------ | --------------------------------- | ------------------------------------------------- |
| `GET`  | `/api/links/graph`                | Full wikilink graph as `{ nodes, edges }`         |
| `GET`  | `/api/links/backlinks/*path`      | Backlinks for one note                            |

Link targets in the graph are the raw `[[target]]` strings as written
— they may be unresolved.

### Scripting

| Method | Path                    | Purpose                                                     |
| ------ | ----------------------- | ----------------------------------------------------------- |
| `POST` | `/api/run`              | Execute a Rhai snippet (body: `{code}`); returns `{output}` |
| `POST` | `/api/render/*path`     | Render a note with its `muninn` code blocks evaluated       |

The render endpoint accepts an optional body:

```json
{ "on_error": "abort" }
```

Values are `abort` (default — first failing block aborts the render)
or `replace_block` (failing blocks swap in an error block so the rest
of the note still renders). Same semantics as the [Scripting](/docs/muninn/scripting/)
`muninn render` CLI.

### Validate

```
POST /api/validate
{ "path": "tasks/t-1.md" }     // omit to validate the whole vault
```

```json
{
  "results": [
    {
      "path": "tasks/t-1.md",
      "errors": [
        { "field": "title", "code": "missing_required", "message": "required field \"title\" is missing", "severity": "error" }
      ]
    }
  ]
}
```

Vault-wide validation returns one entry per note that has at least
one error.

## Using it from other tools

The Tauri desktop app embeds `muninn-core::api::router` in-process —
no sidecar server needed when the app is running. For headless
environments (CI, home servers, Huginn workflows), run `muninn-server`
directly.

Example: Huginn pulling context from Muninn in Python:

```python
import requests

base = "http://127.0.0.1:9200"

# Active tasks
rows = requests.post(f"{base}/api/query", json={
    "sql": "SELECT title, priority FROM task WHERE status = 'active'"
}).json()["rows"]

# Read a specific note
note = requests.get(f"{base}/api/notes/tasks/t-1.md").json()
```

## What's not here

- **Export** (`POST /api/export/...`) lands with Phase 6 alongside
  pandoc + Quarto.
- **Zotero endpoints** land with Phase 6.
- **Auth, TLS, rate limiting** — not planned. This is a localhost
  tool. If you need public access, front it with your own proxy.
- **OpenAPI / schema export** — not yet; this page is the spec.
- **Streaming responses** — everything is a single JSON body.
