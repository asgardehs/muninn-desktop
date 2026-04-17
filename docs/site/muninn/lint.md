---
layout: default
title: Lint
permalink: /docs/muninn/lint/
---

# Lint

`muninn lint` runs a grammar and spell check across notes in your
vault. It uses [harper](https://writewithharper.com/) — a local,
privacy-respecting grammar engine that runs entirely offline.

## Running lint

Check every note in the vault:

```bash
muninn lint
```

Check a single note:

```bash
muninn lint projects/plant-ops.md
```

Check every note of a given type:

```bash
muninn lint --type til
```

Options:

| Flag              | Purpose                                     |
| ----------------- | ------------------------------------------- |
| `--type`, `-t`    | Lint notes of this type only                |

Add `--json` for machine-readable output (one entry per diagnostic
with `path`, `start`, `end`, `message`, `suggestions`, `severity`,
and `rule`).

## Output

```
projects/plant-ops.md
  warn [142..150] Did you mean to spell "recieve" this way? (suggest: receive)
  warn [520..540] This sentence may be wordy. (suggest: Some suggestions)

2 issue(s) in 1 file(s)
```

Each line carries:

- **Severity** — `warn` in the current release; a distinct `error`
  level is wired but not yet assigned by the checker
- **Span** — character offsets into the note body
- **Message** — the diagnostic text
- **Suggestions** — one or more proposed corrections, if harper
  produced any

Frontmatter and fenced code blocks are stripped before linting — only
prose is checked.

## Custom dictionary

Words specific to your domain — project codenames, jargon, proper
nouns — will be flagged unless you add them to the vault dictionary.

Create `.muninn/dictionary.txt` with one word per line:

```text
muninn
heimdall
huginn
btrfs
p&id
```

The file is loaded automatically on each `muninn lint` run. There's no
command to edit it — use any text editor.

## Disabling rules

Grammar rules you don't want to see can be turned off in
`.muninn/config.yaml`:

```yaml
settings:
  grammar:
    disabled_rules:
      - PASSIVE_VOICE
```

A rule's ID is visible in the JSON output under the `rule` field. Copy
it from there.

## Exit code

`muninn lint` always exits 0 regardless of how many diagnostics it
finds — it's a report, not a gate. If you want it to fail builds,
parse the `--json` output and check for any entry with
`severity = "Error"`.

See also: [Configuration](/docs/muninn/configuration/) for the
`settings.grammar` block (enabled, language, `skip_types`,
`disabled_rules`).
