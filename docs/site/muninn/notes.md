---
layout: default
title: Notes
permalink: /docs/muninn/notes/
---

# Notes

Notes are plain markdown files on disk. They live anywhere inside your
vault — organize them in folders however you like, edit them in any
editor, version them with git, and connect them with `[[wikilinks]]`.

## Frontmatter

Notes support YAML frontmatter between `---` fences at the top of the
file:

```yaml
---
title: Btrfs Subvolume Gotchas
type: til
tags:
  - btrfs
  - linux
---
```

Three conventions Muninn recognizes across every vault:

| Field   | Type                  | Used for                                                  |
| ------- | --------------------- | --------------------------------------------------------- |
| `title` | string                | Display name, search-score weighting, wikilink resolution |
| `type`  | string or list        | [Type](/docs/muninn/types/) matching (default key; configurable via `explicit_type_keys`) |
| `tags`  | list of strings       | Search weighting, the `--tag` filter on `list` and `search` |

Every other field is defined by your vault's [types](/docs/muninn/types/).
A note with no frontmatter is still searchable; it just won't have a
declared type or structured fields.

## note new

Create a new note. Muninn slugifies the title to produce the filename
and writes frontmatter populated from your arguments.

```bash
muninn note new "Subvolume permissions" --type til --tags "btrfs,linux"
```

Options:

| Flag                | Purpose                                              |
| ------------------- | ---------------------------------------------------- |
| `--type`, `-t`      | Set the note's `type` field                          |
| `--tags`            | Comma-separated tag list                             |
| `--field KEY=VAL`, `-f` | Set an arbitrary frontmatter field (repeatable)  |

Example with extra fields:

```bash
muninn note new "Incident 2026-04-15" \
  --type incident \
  -f status=active \
  -f severity=high
```

Generated and default fields (from the type's schema) are applied
automatically on creation — UUIDs, timestamps, slugs, and so on.

## note list

List notes in the vault, optionally filtered:

```bash
muninn note list
muninn note list --type til
muninn note list --tag linux
muninn note list --type til --tag linux --title subvolume
```

Options:

| Flag              | Purpose                                      |
| ----------------- | -------------------------------------------- |
| `--type`, `-t`    | Only notes with this type                    |
| `--tag`           | Only notes with this tag                     |
| `--title`         | Only notes whose title contains this substring (case-insensitive) |

Add `--json` for machine-readable output.

## note search

Full-text search with the same filters as `note list`. See
[Search](/docs/muninn/search/) for the scoring model.

```bash
muninn note search "btrfs subvolume"
muninn note search "context cancellation" --type til --tag go
```

`muninn search "..."` is a shortcut for `muninn note search "..."`.

## note backlinks

Show every note that links to a given note:

```bash
muninn note backlinks subvolume-permissions.md
```

Output:

```
Notes linking to subvolume-permissions.md:
  linux-filesystem-notes.md
  journal/2026-04-15.md
```

## Wikilinks

Connect notes with `[[wikilinks]]`:

| Syntax                               | Meaning                                                 |
| ------------------------------------ | ------------------------------------------------------- |
| `[[target]]`                         | Link to `target.md`                                     |
| `[[target\|display text]]`           | Link to `target.md`, render "display text"              |
| `[[target#heading]]`                 | Link to a heading in `target.md`                        |
| `[[target#heading\|display text]]`   | Link to heading, render "display text"                  |
| `[[folder/]]`                        | Link to a folder (trailing slash)                       |
| `![[target]]`                        | Embed the target (attachment preview or transclusion)   |

Target matching is case-insensitive and resolves against filenames
without the `.md` extension. Folders, notes, and attachments live in
the same link graph — a `[[folder/]]` link is as much a node as a note
is.

Folders can carry their own description in an `_index.md` file. The
index file is not listed by `note list` but is the body that a
`[[folder/]]` link resolves to.

## Attachments

Non-markdown files — images, PDFs, diagrams — live in `_attachments/`
directories alongside related notes:

```
projects/
├── plant-ops.md
└── _attachments/
    ├── diagram.png
    └── p&id-v3.pdf
```

`_attachments/` directories are skipped by `note list`, `note search`,
and any other operation that scans notes. They're still reachable via
embeds:

```markdown
![[diagram.png]]
```

Embeds in the desktop preview (Phase 7) will render images inline. In
the CLI, an embed is just a link — the rendering is done by whatever
tool displays the note.

See also: [Types](/docs/muninn/types/) for field schemas and
validation, [Search](/docs/muninn/search/) for how full-text scoring
works.
