---
layout: default
title: Muninn Documentation
permalink: /docs/muninn/
---

# Muninn Documentation

Muninn is a local-first personal knowledge base. Notes are plain markdown
files on disk — no database, no cloud, no external runtime. Typed
frontmatter (mdbase), SQL queries, inline scripting, wikilinks, grammar
check, and a desktop app built in.

## Getting Started

- [Quick Start](/docs/muninn/quickstart/) — install, create a vault, write your first note
- [Requirements](/docs/muninn/requirements/) — what you need to build and run Muninn
- [Configuration](/docs/muninn/configuration/) — vault paths, `.muninn/` layout, environment variables

## Reference

- [Command Reference](/docs/muninn/commands/) — every CLI subcommand at a glance
- [Notes](/docs/muninn/notes/) — create, list, search, wikilinks, folder links, attachments
- [Types](/docs/muninn/types/) — mdbase type definitions, validation, inheritance
- [Query](/docs/muninn/query/) — SQL over frontmatter with `muninn query`
- [Scripting](/docs/muninn/scripting/) — Rhai script blocks and `.rhai` files
- [Search](/docs/muninn/search/) — text search across your vault
- [Lint](/docs/muninn/lint/) — grammar and spell check

Muninn is part of the [Asgard EHS ecosystem](/). It's designed to stand
alone but integrates with Huginn and Odin through a local HTTP API.
