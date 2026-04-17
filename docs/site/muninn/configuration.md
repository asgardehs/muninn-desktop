---
layout: default
title: Configuration
permalink: /docs/muninn/configuration/
---

# Configuration

Muninn is configured through two things: the `MUNINN_VAULT_PATH`
environment variable, and a `.muninn/config.yaml` file inside the
vault.

## Vault Path Resolution

When you run any `muninn` command, it resolves the vault directory in
this order:

1. The `MUNINN_VAULT_PATH` environment variable, if set
2. `$XDG_DATA_HOME/muninn`, if `XDG_DATA_HOME` is set
3. The platform default: `~/.local/share/muninn`

### Setting via environment variable

```bash
export MUNINN_VAULT_PATH=~/notes
```

Add this to your shell profile (`~/.bashrc`, `~/.zshrc`, `~/.profile`,
Fish's `config.fish`, etc.) so it's available in every session.

## Vault Structure

A vault is a directory of markdown notes plus a `.muninn/` subdirectory
for configuration. After running `muninn init`:

```
~/notes/
├── .muninn/
│   ├── config.yaml             # collection settings
│   ├── types/                  # mdbase type definitions
│   │   └── note.md
│   ├── scripts/                # reusable .rhai scripts (optional)
│   └── dictionary.txt          # custom dictionary (optional)
└── <your notes>.md
```

Notes can live at any depth below the vault root. Folders are just
folders — organize however you like. Two conventions Muninn recognizes:

- **`_attachments/`** — any directory named `_attachments` is skipped
  during note scanning. Use these for images, PDFs, and other
  non-markdown files referenced from your notes.
- **`_index.md`** — folder metadata files. They're not included in
  `note list` output but are linkable via `[[folder/]]`.

Run `git init` inside the vault directory to start versioning your notes.

## `.muninn/config.yaml`

The config file controls collection-wide settings. A minimal config
looks like this:

```yaml
spec_version: "0.2.0"
name: my-vault
```

All fields:

| Field                                | Type              | Default     | Purpose                                                |
| ------------------------------------ | ----------------- | ----------- | ------------------------------------------------------ |
| `spec_version`                       | string            | `"0.2.0"`   | mdbase spec version this vault targets                 |
| `name`                               | string            | `""`        | Human-readable vault name                              |
| `description`                        | string            | _(omitted)_ | One-line description                                   |
| `settings.explicit_type_keys`        | list of strings   | `[]`        | Frontmatter keys Muninn checks for type declarations (falls back to `type` and `types` when empty) |
| `settings.grammar.enabled`           | boolean           | `true`      | Whether `muninn lint` runs                             |
| `settings.grammar.language`          | string            | `"en-US"`   | Language code for the grammar checker                  |
| `settings.grammar.skip_types`        | list of strings   | `[]`        | Type names to skip during lint (e.g. `["quarto"]`)     |
| `settings.grammar.disabled_rules`    | list of strings   | `[]`        | Grammar rules to disable by ID                         |

A fuller example:

```yaml
spec_version: "0.2.0"
name: knowledge-base
description: personal and work notes
settings:
  explicit_type_keys:
    - type
    - kind
  grammar:
    enabled: true
    language: en-US
    skip_types:
      - quarto
    disabled_rules:
      - PASSIVE_VOICE
```

## Custom Dictionary

`muninn lint` wraps the [harper](https://writewithharper.com/) grammar
and spell checker. Words specific to your domain — project codenames,
jargon, people's names — will be flagged as misspellings unless you
add them to the vault dictionary.

Create `.muninn/dictionary.txt` with one word per line:

```text
muninn
heimdall
huginn
odin
btrfs
```

The dictionary is loaded automatically when `muninn lint` runs.

## Types Directory

Type definitions live in `.muninn/types/<name>.md`. Each file describes
one type's fields, constraints, and matching rules. See
[Types](/docs/muninn/types/) for the full format.

After `muninn init`, the directory contains a starter `note.md` type
you can edit or replace.

## Scripts Directory

Reusable `.rhai` scripts live in `.muninn/scripts/<name>.rhai`. Notes
can import them with `import "name" as alias;`. See
[Scripting](/docs/muninn/scripting/) for the script language and
vault API.

See also: [Requirements](/docs/muninn/requirements/) for platform
support, [Quick Start](/docs/muninn/quickstart/) for the first-run
walkthrough.
