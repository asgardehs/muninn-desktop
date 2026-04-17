---
layout: default
title: Search
permalink: /docs/muninn/search/
---

# Search

Muninn's text search ranks every note in the vault against your query
and returns the top matches. For "find me a note about X" questions,
this is usually what you want. For structured questions over typed
fields, reach for [Query](/docs/muninn/query/) instead.

## How it works

The query is split on whitespace into lowercase words. Each word is
scored against each note with simple case-insensitive substring
matching:

- **Title match** — 3 points per query word
- **Tag match** — 2 points per query word
- **Body match** — 1 point per query word

A word that appears in all three places scores 6 for that note.
Results are ranked by total score descending; the top `--limit`
matches are returned with a snippet line showing surrounding body
context.

## search

Top-level shortcut:

```bash
muninn search "btrfs subvolume permissions"
```

With filters:

```bash
muninn search "context cancellation" --type til --tag go
```

Options:

| Flag              | Purpose                                     |
| ----------------- | ------------------------------------------- |
| `--limit`         | Maximum results (default: 10)               |
| `--type`, `-t`    | Only notes of this type                     |
| `--tag`           | Only notes with this tag                    |

Add `--json` for machine-readable output (`path`, `title`, `score`,
`snippet`).

### Output

```
[projects/plant-ops.md] Plant Operations (score: 7)
  subvolume permissions for btrfs snapshots are covered...
---
[til/btrfs-quota.md] Btrfs Quota Groups (score: 3)
  ...
```

Each result is separated by `---`. The number in brackets is the
vault-relative path; the parenthesized score lets you see how strongly
a result matched.

## note search

`muninn note search` is the same command under the `note` subcommand
tree — use whichever feels more natural. The flags are identical.

## Tips

- **Use multiple words.** Each word contributes to the score
  independently. "btrfs subvolume permissions" gives three chances to
  match per note; "btrfs" gives one.
- **Filter before searching.** When you know the type or tag you're
  after, `--type` and `--tag` skip notes outside that slice entirely
  — both faster and less noisy.
- **Substring, not tokenized.** "subvol" matches "subvolume";
  "sub volume" does not match "subvolume". Pick search terms that
  actually appear in your notes.

See also: [Query](/docs/muninn/query/) for SQL over frontmatter,
[Notes](/docs/muninn/notes/) for the `list` and `backlinks` commands.
