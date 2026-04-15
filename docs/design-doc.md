---
project: Muninn
description:
  A personal knowledge base and note management tool
doc_date: 2026-03-31
doc_rev_date: 2026-04-06
supersedes:
written_by: Adam Bick
---

# Muninn — Design Document

_A personal knowledge base and note management tool._
_"Odin's raven, who flies over the world gathering memory."_

## Overview

Muninn manages a vault of markdown notes on disk and makes them searchable.
Notes are plain files — edit them in any editor, version with git, and link them
together with `[[wikilinks]]`. No database, no cloud, no external dependencies
beyond Go.

Two interfaces share the same vault:

- **CLI** (`muninn note new`, `muninn search`, etc.) — create, list, and search
  notes from the terminal
- **LSP Server** (`muninn lsp`) — editor integration for wikilink navigation,
  backlinks, completions, and diagnostics

## Architecture

```ascii
┌──────────────┐  ┌──────────────┐
│   CLI        │  │  LSP Server  │
│  (cobra)     │  │  (stdio)     │
└──────┬───────┘  └──────┬───────┘
       │                 │
       └────────┬────────┘
                │
       ┌────────▼─────────┐
       │      vault       │
       │                  │
       │  ListNotes()     │
       │  ReadNote()      │
       │  CreateNote()    │
       │  Search()        │
       │  ListFiltered()  │
       │  CollectTags()   │
       └───────┬──────────┘
               │
      ┌────────┼──────────────┐
      │        │              │
  ┌───▼────┐ ┌─▼──────────┐ ┌▼────────────┐
  │markdown│ │  wikilink   │ │  filesystem  │
  │        │ │             │ │              │
  │ Parse()│ │ Extract()   │ │  notes/*.md  │
  │ Front..│ │ Index       │ │              │
  │ Title()│ │ Backlinks() │ │              │
  └────────┘ └─────────────┘ └──────────────┘
```

## Data Location

`muninn init` creates a single directory:

```ascii
~/.local/share/muninn/
└── notes/             # markdown vault (git-tracked)
    ├── some-topic.md
    ├── go-context-patterns.md
    ├── journal/
    │   ├── 2026-04-W1-05.md
    │   └── ...
    └── ...
```

No database, no generated files. Notes are the source of truth. Run `git init`
in the vault to version everything.

## Search

Text search walks every note in the vault and scores against the query:

- **Title match** — 3 points per query word found in the title
- **Tag match** — 2 points per query word matching a frontmatter tag
- **Body match** — 1 point per query word found in the note content

Results are ranked by total score. Frontmatter filters (type, status, area,
project, language) narrow results before scoring.

## Frontmatter

Notes support YAML frontmatter between `---` fences:

| Field      | Type   | Description                  |
| ---------- | ------ | ---------------------------- |
| `title`    | string | Note title                   |
| `type`     | enum   | design-doc, til, reference, decision, troubleshooting, journal |
| `status`   | enum   | draft, active, complete, archived |
| `area`     | enum   | personal, work, journal      |
| `project`  | string | Associated project           |
| `language` | string | Primary programming language |
| `tags`     | list   | Categorization tags          |

All fields optional. Parsed by walking the YAML and flattening to key-value
entries (arrays expanded, nested maps use dot notation).

## Wikilink System

Wikilinks create a navigable knowledge graph:

- **Syntax:** `[[target]]`, `[[target|alias]]`, `[[target#heading]]`
- **Resolution:** target normalized (lowercase, trimmed), matched against
  filenames without `.md`
- **In-memory index:** bidirectional — forward links and backlinks, built by
  scanning the vault at startup
- **Thread-safe:** `sync.RWMutex` on the index, updated on file save

The LSP uses this index for completions, go-to-definition, backlinks, hover,
rename, code lens, and diagnostics.

## LSP Server

Communicates over stdio. Capabilities:

- Completions (note names, headings, tags)
- Go to Definition (wikilinks, heading fragments)
- References (backlinks)
- Hover (note preview)
- Diagnostics (broken links)
- Document/Workspace Symbols
- Semantic Tokens (resolved/broken link highlighting)
- Code Lens (reference counts)
- Code Actions (create note from broken link)
- Rename (note/heading with cascading updates)

## Project Structure

```ascii
github.com/adambick/muninn/
├── cmd/
│   └── muninn/
│       ├── main.go            # Cobra root: note, search, lsp, init, install
│       ├── env.go             # Vault path helpers
│       ├── cmd_note.go        # note new/list/search/backlinks
│       ├── cmd_search.go      # Top-level search
│       ├── cmd_lsp.go         # LSP server launcher
│       ├── cmd_init.go        # Vault setup
│       └── cmd_install.go     # VS Code extension install
├── internal/
│   ├── vault/
│   │   ├── vault.go           # File I/O: list, read, create, rename notes
│   │   ├── search.go          # Text search across vault
│   │   ├── list.go            # Filtered note listing
│   │   ├── tags.go            # Tag collection from frontmatter
│   │   └── vault_test.go
│   ├── markdown/
│   │   ├── parser.go          # Goldmark: frontmatter + title extraction
│   │   ├── frontmatter.go     # YAML → flat key-value entries
│   │   ├── heading.go         # Heading extraction
│   │   ├── schema.go          # Note template rendering
│   │   └── parser_test.go
│   ├── wikilink/
│   │   ├── extract.go         # Regex extraction from markdown
│   │   ├── index.go           # In-memory bidirectional link graph
│   │   └── extract_test.go
│   └── lsp/
│       ├── server.go          # LSP server setup + document sync
│       ├── completion.go      # Wikilink + tag completions
│       ├── definition.go      # Go-to-definition
│       ├── references.go      # Backlinks
│       ├── hover.go           # Note preview
│       ├── rename.go          # Cascading rename
│       ├── symbols.go         # Document + workspace symbols
│       ├── tokens.go          # Semantic tokens
│       ├── codelens.go        # Reference count lens
│       ├── codeactions.go     # Quick fix: create missing note
│       ├── diagnostics.go     # Broken link warnings
│       ├── commands.go        # Create note, daily note, graph links
│       └── lsp_test.go
├── vscode/
│   ├── package.json           # Extension manifest
│   ├── src/extension.ts       # Launch muninn lsp, wire to markdown
│   └── syntaxes/              # TextMate grammar for wikilinks
├── go.mod
├── go.sum
├── Makefile
├── LICENSE                    # GPLv3
└── README.md
```

## Dependencies

```
github.com/spf13/cobra         # CLI framework
github.com/yuin/goldmark        # Markdown parsing (CommonMark + GFM)
go.lsp.dev/jsonrpc2             # JSON-RPC 2.0 transport
go.lsp.dev/protocol             # LSP protocol types
go.lsp.dev/uri                  # URI handling
gopkg.in/yaml.v3                # YAML frontmatter parsing
```

No CGO required. Pure Go build.

## CLI Commands

```bash
# Notes
muninn note new "Some Topic" --type til --tags "go,concurrency"
muninn note list --type reference --area work
muninn note search "error handling patterns"
muninn note backlinks some-topic.md

# Search (top-level alias for note search)
muninn search "btrfs subvolume permissions"
muninn search "context cancellation" --lang go --type til

# Setup
muninn init
muninn install      # VS Code extension
muninn lsp          # stdio LSP server (managed by VS Code)
```

## Configuration

Environment variable only:

```
MUNINN_VAULT_PATH   — override default vault location (~/.local/share/muninn)
```

## History

Muninn was originally built with SQLite (+ sqlite-vec), ONNX Runtime local
embeddings, and an MCP server for AI assistant integration. This was
over-engineered for a personal tool. On 2026-04-06 it was simplified to flat
files with text search. The archive of the original architecture is preserved
at `~/media/projects/asgard/muninn-archive.tar`.
