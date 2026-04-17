---
layout: default
title: Quick Start
permalink: /docs/muninn/quickstart/
---

# Quick Start

This walks you from zero to a working vault with a typed note you can
query. It takes about five minutes.

## Install

Muninn builds with a current stable Rust toolchain (see
[Requirements](/docs/muninn/requirements/)). Clone the repository and
install the CLI:

```bash
git clone https://github.com/asgardehs/muninn-desktop.git
cd muninn-desktop
cargo install --path crates/muninn-cli
```

This produces a `muninn` binary in `~/.cargo/bin/`. Confirm it's on your
PATH:

```bash
muninn --version
```

## Create a vault

A vault is a directory of markdown files plus a `.muninn/` subdirectory
for configuration and types. Pick a location and point `muninn` at it:

```bash
export MUNINN_VAULT_PATH=~/notes
muninn init
```

`init` creates the directory (if it doesn't exist) along with a minimal
`.muninn/config.yaml`. See [Configuration](/docs/muninn/configuration/)
for the full layout and platform default paths.

## First Steps

### Create a note

```bash
muninn note new "Subvolume permissions" --type note --tags "linux,btrfs"
```

Muninn writes `~/notes/subvolume-permissions.md` with frontmatter
pre-filled from your arguments. Open it in any editor and add a body.

### Find it later

Full-text search scores matches across title, tags, and body:

```bash
muninn search "btrfs subvolume"
```

### List your notes

```bash
muninn note list --type note --tag linux
```

### Check backlinks

When another note contains `[[subvolume-permissions]]`, this shows up:

```bash
muninn note backlinks subvolume-permissions.md
```

### Query structured data

Once notes have consistent frontmatter, query across them with SQL:

```bash
muninn query "SELECT title, status FROM note WHERE status = 'active'"
```

See [Query](/docs/muninn/query/) for the full SQL dialect and built-in
functions.

### Evaluate a script block

Add a `muninn` fenced block to any note and run it through `render`:

````markdown
# Dashboard

```muninn
let active = query("SELECT COUNT(*) AS n FROM note WHERE status = 'active'");
print(`Active notes: ${active[0].n}`);
```
````

```bash
muninn render ~/notes/dashboard.md
```

See [Scripting](/docs/muninn/scripting/) for the full script API.

## Next

- Define a [Type](/docs/muninn/types/) to validate frontmatter across notes
- Browse the full [Command Reference](/docs/muninn/commands/)
- Learn the [Search](/docs/muninn/search/) scoring model
