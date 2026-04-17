---
layout: default
title: Scripting
permalink: /docs/muninn/scripting/
---

# Scripting

Muninn has a small embedded scripting engine so you can compute over
your notes from inside a note. Scripts can query the vault, read notes,
and emit formatted output (markdown tables, lists, links). They cannot
create or modify notes — scripts are for viewing and computing, not for
side effects. Mutation happens through the CLI or the desktop app.

If this is your first time writing code, this page will get you from
nothing to a useful weekly-summary template. No prior programming
experience assumed.

## How it works

There are two places scripts live:

- **Inside a note**, as a fenced code block with the language tag
  `muninn`. When you run `muninn render <note>`, each block is
  evaluated and its output replaces the block in the rendered output.
- **As a standalone file** ending in `.rhai`, stored in
  `.muninn/scripts/` inside your vault. You can import these from notes
  or run them directly with `muninn run <file>`.

The language is [Rhai](https://rhai.rs) — a small Rust-like scripting
language that ships inside the `muninn` binary. You don't need to
install anything extra.

Each time a block runs, it gets a fresh engine. Nothing carries over
between blocks, notes, or renders.

## Quick tour: build a weekly summary

We'll build up a "Weekly Summary" note piece by piece. At each step you
can run `muninn render weekly-summary.md` and see the current state.

Start by creating an empty note in your vault:

```bash
muninn note new "Weekly Summary" --type note
```

### 1. Print something

Open `weekly-summary.md` in your editor and paste:

````markdown
# Weekly Summary

```muninn
print("Hello from a script.");
```
````

Now render it:

```bash
muninn render ~/notes/weekly-summary.md
```

The output replaces the fenced block:

```
# Weekly Summary

Hello from a script.
```

`print(x)` writes text to the block's output buffer. Every `print`
call adds a newline.

### 2. Query the vault

Replace the block with:

```muninn
let active = query("SELECT title, status FROM note WHERE status = 'active'");
print(`Active notes: ${active.len()}`);
```

Three new ideas in three lines:

- **`let name = ...`** declares a variable. Variables hold values you
  compute once and use later.
- **`query("...")`** runs a SQL query against your vault and returns
  the matching rows. See [Query](/docs/muninn/query/) for the SQL
  dialect.
- **`` `...${x}...` ``** — backticks around a string let you embed a
  value with `${x}`. Without backticks, quotes are literal.

`active.len()` is the number of rows the query returned.

### 3. Render as a table

Counting is useful, but a table is more useful. Swap the block for:

```muninn
let active = query("SELECT title, status, updated FROM note WHERE status = 'active' ORDER BY updated DESC");
table(active);
```

`table(rows)` emits a markdown pipe table. Column headers come from the
query — whatever you selected becomes the columns.

### 4. Show different things based on the data

Empty sections are noise. Let's only show a list if it has content:

```muninn
let overdue = query("SELECT title, due FROM note WHERE due < TODAY()");
if overdue.len() > 0 {
    print("## Overdue");
    table(overdue);
} else {
    print("_No overdue items this week._");
}
```

- **`if condition { ... } else { ... }`** runs the first block if
  the condition is true, otherwise the second. The `else` part is
  optional.
- **`TODAY()`** is a SQL function that returns today's date. See
  [Query](/docs/muninn/query/) for the full list.

### 5. Loop over rows

When you need one line per row rather than a whole table, use a loop:

```muninn
let recent = query("SELECT title, date FROM journal ORDER BY date DESC LIMIT 5");
print("## Recent Journal Entries");
for j in recent {
    print(`- [[${j.title}]] (${j.date})`);
}
```

- **`for name in list { ... }`** runs the body once per item. Here, `j`
  is one row at a time.
- **`j.title`** pulls the `title` column out of that row.
- **`[[...]]`** is a wikilink. The rendered output will be a real link
  Muninn recognizes.

### 6. Extract a helper (functions)

The block is getting long. Anything you write more than once is a good
candidate to pull out into a function:

```muninn
fn section(heading, rows) {
    if rows.len() == 0 { return; }
    print(`## ${heading}`);
    table(rows);
}

let active = query("SELECT title, status FROM note WHERE status = 'active'");
let overdue = query("SELECT title, due FROM note WHERE due < TODAY()");

section("Active", active);
section("Overdue", overdue);
```

- **`fn name(args) { ... }`** declares a function. Inside the braces,
  `args` are placeholders that get filled in each time you call it.
- **`return;`** exits the function early. We use it here to skip the
  heading and empty table when there's nothing to show.

### 7. Move the helpers into a file

If you'll use the same helpers across multiple notes, put them in a
`.rhai` file instead of repeating them. Create
`.muninn/scripts/weekly.rhai` in your vault:

```rhai
fn active_projects() {
    query("SELECT title, status, updated FROM note WHERE status = 'active' ORDER BY updated DESC")
}

fn overdue_items() {
    query("SELECT title, due FROM note WHERE due < TODAY() ORDER BY due")
}

fn section(heading, rows) {
    if rows.len() == 0 { return; }
    print(`## ${heading}`);
    table(rows);
}
```

Now your note block shrinks to:

```muninn
import "weekly" as w;

w::section("Active Projects", w::active_projects());
w::section("Overdue", w::overdue_items());
```

- **`import "name" as alias;`** loads `.muninn/scripts/name.rhai`. The
  file extension and path are implicit.
- **`alias::function()`** calls a function from the imported file.

### 8. The finished template

Putting it all together, your `weekly-summary.md` might look like this:

````markdown
# Weekly Summary

```muninn
import "weekly" as w;

w::section("Active Projects", w::active_projects());
w::section("Overdue", w::overdue_items());

let recent = query("SELECT title, date FROM journal ORDER BY date DESC LIMIT 5");
print("## Recent Journal Entries");
for j in recent {
    print(`- [[${j.title}]] (${j.date})`);
}
```
````

Run `muninn render ~/notes/weekly-summary.md` anytime — the output
reflects the current state of your vault.

## Vault API

Read functions your scripts can call:

| Function                 | Returns                                         |
| ------------------------ | ----------------------------------------------- |
| `query(sql)`             | Array of rows matching a SQL query              |
| `search(text)`           | Array of notes scored by full-text match        |
| `note(path)`             | A single note with frontmatter, body, and tags  |
| `notes()` / `notes(f)`   | All notes, optionally filtered by `type`/`tag`  |
| `backlinks(path)`        | Notes that link to the given note               |
| `tags()`                 | Every tag with its use count                    |
| `types()`                | Every mdbase type defined in the vault          |

Every function that takes a path accepts a vault-relative path like
`"projects/plant-ops.md"`.

Rows returned by `query()` and `notes()` always include a synthetic
`path` column pointing to the source note in the vault — handy when you
want to link back to where a row came from:

```muninn
let rows = query("SELECT title FROM note WHERE status = 'active'");
for r in rows {
    print(`- [[${r.title}]] — ${r.path}`);
}
```

## Output functions

These write into the block's output buffer. Whatever ends up there
replaces the fenced block after rendering:

| Function          | Emits                                             |
| ----------------- | ------------------------------------------------- |
| `print(text)`     | A line of raw text                                |
| `table(rows)`     | A markdown pipe table                             |
| `list(items)`     | A markdown bullet list                            |
| `link(path)`      | A wikilink like `[[path]]` (returns a string)     |
| `json(value)`     | A JSON-stringified version of a value             |

`link` and `json` return strings — use them inside `print` or
string templates.

## When a block fails

By default, if any `muninn` block in a note throws an error,
`muninn render` aborts with a non-zero exit code so the failure is
visible:

```bash
$ muninn render weekly-summary.md
error: rendering weekly-summary.md: eval error: ...
```

Pass `--continue-on-error` to keep rendering. Each failing block is
replaced in the output with a fenced `muninn-error` block containing
the error message:

```bash
muninn render weekly-summary.md --continue-on-error
```

## Resource limits

Every run is bounded so a runaway script can't lock up your terminal or
the desktop app:

- Maximum **1,000,000 operations** per run
- Maximum **64** levels of function-call depth
- **5-second** wall-clock timeout

Typical summary or dashboard scripts finish in under a millisecond. If
you're brushing up against a limit, your script either has a bug (an
infinite loop) or is doing more work than belongs in a render block —
move it to the CLI or a scheduled job.

## Further reading

- [Rhai Language Reference](https://rhai.rs/book/language/) — complete
  syntax for variables, types, operators, and control flow
- [Query](/docs/muninn/query/) — the SQL dialect used by `query()`
- [Notes](/docs/muninn/notes/) — wikilinks, frontmatter, folder links
