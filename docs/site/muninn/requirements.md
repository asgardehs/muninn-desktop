---
layout: default
title: Requirements
permalink: /docs/muninn/requirements/
---

# Requirements

Muninn is a single statically-linked binary with no runtime
dependencies. No Python, no Node, no external tools. Your vault is
plain markdown on disk — version it with git if you want history.

## Running

- A supported platform: **Linux**, **macOS**, or **Windows**
- Read/write access to a directory for your vault

That's it. The binary includes the SQL query engine, the Rhai
scripting runtime, the grammar checker, and everything else.

## Building from source

- **Rust toolchain**, stable, **1.85 or later** (the workspace uses
  Rust edition 2024)
- Git, to clone the repository

Install via `rustup`:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Then:

```bash
git clone https://github.com/asgardehs/muninn-desktop.git
cd muninn-desktop
cargo install --path crates/muninn-cli
```

See [Quick Start](/docs/muninn/quickstart/) for the first-run walkthrough.

## Optional

- **Git** inside your vault — Muninn doesn't require it, but since
  notes are plain files, versioning with git works naturally.
- A text editor of your choice. Any editor that handles markdown
  and YAML is fine.
