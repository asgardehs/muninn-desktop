---
layout: default
title: Command Reference
permalink: /docs/muninn/commands/
---

# Command Reference

The `muninn` binary is the single entry point for all vault operations.
Every command operates on the vault resolved from `MUNINN_VAULT_PATH` or
the platform default (see [Configuration](/docs/muninn/configuration/)).

| Command              | Description                                       | Page                                   |
| -------------------- | ------------------------------------------------- | -------------------------------------- |
| `init`               | Create a new vault directory                      | [Quick Start](/docs/muninn/quickstart/) |
| `note new`           | Create a new note                                 | [Notes](/docs/muninn/notes/)           |
| `note list`          | List notes, optionally filtered                   | [Notes](/docs/muninn/notes/)           |
| `note search`        | Search notes with frontmatter filters             | [Notes](/docs/muninn/notes/)           |
| `note backlinks`     | Show notes linking to a given note                | [Notes](/docs/muninn/notes/)           |
| `search`             | Full-text search across the vault                 | [Search](/docs/muninn/search/)         |
| `type list`          | List all defined mdbase types                     | [Types](/docs/muninn/types/)           |
| `type show`          | Show a type's fields and inheritance              | [Types](/docs/muninn/types/)           |
| `validate`           | Validate notes against their type schemas         | [Types](/docs/muninn/types/)           |
| `query`              | Run a SQL query over frontmatter                  | [Query](/docs/muninn/query/)           |
| `runestone list`     | List saved Runestone views                        | [Runestones](/docs/muninn/runestones/) |
| `runestone show`     | Print a Runestone's YAML definition               | [Runestones](/docs/muninn/runestones/) |
| `runestone eval`     | Evaluate a Runestone and print its rows           | [Runestones](/docs/muninn/runestones/) |
| `run`                | Execute a `.rhai` script against the vault        | [Scripting](/docs/muninn/scripting/)   |
| `render`             | Render a note with its `muninn` blocks evaluated  | [Scripting](/docs/muninn/scripting/)   |
| `lint`               | Check grammar and spelling                        | [Lint](/docs/muninn/lint/)             |

## Global flags

| Flag        | Effect                                              |
| ----------- | --------------------------------------------------- |
| `--json`    | Emit machine-readable JSON where the command supports it |
| `--help`    | Print help for the command or subcommand            |
| `--version` | Print the binary version                            |

The `--json` flag is honored by every command that produces listable
output: `note list`, `note search`, `search`, `type list`, `type show`,
`validate`, `query`, and `runestone`.

## Help

Every subcommand accepts `--help`:

```bash
muninn --help              # top-level command list
muninn note --help         # subcommands of `note`
muninn query --help        # flags for `query`
```

## Related binaries

The vault ships with a second binary:

| Binary          | Purpose                                                            | Page                                 |
| --------------- | ------------------------------------------------------------------ | ------------------------------------ |
| `muninn-server` | Standalone HTTP API for the same operations, used by other Asgard tools | [HTTP API](/docs/muninn/http-api/)   |

See also: [Configuration](/docs/muninn/configuration/) for environment
variables and default paths.
