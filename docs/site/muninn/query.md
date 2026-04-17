---
layout: default
title: Query
permalink: /docs/muninn/query/
---

# Query

`muninn query` runs SQL over the typed frontmatter in your vault. Same
engine is available from scripts as the `query()` function — see
[Scripting](/docs/muninn/scripting/).

If your frontmatter is consistent (ideally via [types](/docs/muninn/types/)),
SQL gives you direct answers to questions like "what's active?", "which
design docs haven't been updated this month?", or "how many notes by
status?". For unstructured "find me a note about X" questions,
[Search](/docs/muninn/search/) is the better tool.

## Running a query

```bash
muninn query "SELECT title, status FROM note WHERE status = 'active'"
```

Output is a table by default; add `--json` for machine-readable output,
or `--with-path` to include the source note's path as the first column.

From inside a note:

````markdown
```muninn
let rows = query("SELECT title, status FROM note WHERE status = 'active'");
table(rows);
```
````

## Quick tour

### Every note, by type

The `FROM` clause is a type name from `.muninn/types/`. Only notes
that match that type appear:

```sql
SELECT title, mood FROM journal
```

To select across every note regardless of type, use the special table
name `note`:

```sql
SELECT title, type FROM note
```

This works even if you haven't defined a `note` type — `FROM note` is
a synthetic any-type source. Every other `FROM` value must be a real
type.

### Filter with WHERE

```sql
SELECT title, status FROM note
WHERE status = 'active' AND tags IS NOT NULL
```

Supported operators: `=`, `!=`, `<>`, `<`, `<=`, `>`, `>=`, `AND`,
`OR`, `NOT`, `IN`, `BETWEEN`, `LIKE`, `IS NULL`, `IS NOT NULL`.

### Sort and paginate

```sql
SELECT title, updated FROM note
ORDER BY updated DESC
LIMIT 10 OFFSET 20
```

`ORDER BY` resolves column aliases, so this works:

```sql
SELECT UPPER(title) AS t FROM note ORDER BY t
```

### Join across types

Give each `FROM` source an alias and reference columns via `alias.column`.
Bare column names still resolve against the primary (first) alias.

```sql
SELECT t.title, p.title, p.status
FROM task t JOIN project p ON t.project = p.path
WHERE t.status = 'active'
```

`LEFT JOIN` keeps rows from the left side even when nothing on the
right matches — the right-side columns come back as NULL, which pairs
well with `IS NULL` to find orphans:

```sql
SELECT t.title
FROM task t LEFT JOIN project p ON t.project = p.path
WHERE p.title IS NULL
```

Self-joins work: alias the same type twice and join by whichever link
field points between them. A common pattern is rolling up a parent
note's fields onto its children:

```sql
SELECT a.title, b.title AS parent_title, b.status AS parent_status
FROM task a JOIN task b ON a.parent = b.path
```

Join keys are standard SQL equality over whatever is in the
frontmatter. If your link fields hold wikilink text like
`[[other-note]]`, store the resolved path (`other-note.md`) instead so
`ON a.parent = b.path` matches — wikilink-aware resolution is not
wired into joins yet.

### Group and aggregate

```sql
SELECT status, COUNT(*) AS n
FROM note
GROUP BY status
HAVING COUNT(*) >= 2
ORDER BY n DESC
```

## Supported clauses

| Clause           | Notes                                                  |
| ---------------- | ------------------------------------------------------ |
| `SELECT`         | Columns, expressions, aliases, `*`                     |
| `FROM`           | A type name with optional alias, or the synthetic `note` source |
| `JOIN ... ON`    | `INNER` and `LEFT`; self-joins via aliases             |
| `WHERE`          | Row filter                                             |
| `GROUP BY`       | Aggregate by one or more expressions                   |
| `HAVING`         | Filter on aggregates                                   |
| `ORDER BY`       | `ASC` / `DESC`; resolves projection aliases            |
| `LIMIT` / `OFFSET` | Pagination                                           |

Not supported in this version:

- **`JOIN USING`, `NATURAL JOIN`, `RIGHT`, `FULL`** — use `JOIN ... ON` explicitly.
- **Subqueries** — no `SELECT ... WHERE x IN (SELECT ...)`.
- **`UNION`, `INTERSECT`, `EXCEPT`** — no set operations.
- **DDL / DML** — `CREATE`, `INSERT`, `UPDATE`, `DELETE` are all
  rejected. The query engine is strictly read-only; note mutation
  goes through the CLI or the desktop app.

## Columns

Any field from the matched type's frontmatter is a column. Three
synthetic columns are available on every note:

| Column  | Value                                         |
| ------- | --------------------------------------------- |
| `path`  | Vault-relative path to the note               |
| `title` | The note's resolved title                     |
| `tags`  | The note's tag list (as a list value)         |

If a type defines [computed fields](/docs/muninn/types/#computed-fields),
those appear as columns too — referenced the same way as real
frontmatter. Computed fields re-evaluate per row, so they compose with
`WHERE`, `ORDER BY`, and `SELECT` expressions.

## Scalar functions

Evaluated once per row. Use in `SELECT`, `WHERE`, `HAVING`, `ORDER BY`:

| Function                 | Returns                                         |
| ------------------------ | ----------------------------------------------- |
| `TODAY()`                | Today's date                                    |
| `NOW()`                  | Current datetime                                |
| `YEAR(date)`             | Year component of a date/datetime               |
| `LENGTH(x)`              | Character count for strings, element count for lists |
| `LOWER(s)`, `UPPER(s)`   | Case conversion for strings                     |
| `COALESCE(a, b, ...)`    | First non-null argument                         |
| `DATE_ADD(d, days)`      | Add days to a date or datetime                  |
| `EXISTS(x)`              | True if `x` is non-null and non-empty           |

## Aggregate functions

Used with `GROUP BY`, or on the whole result set when there's no
`GROUP BY` but an aggregate appears in `SELECT`:

| Function     | Returns                                                  |
| ------------ | -------------------------------------------------------- |
| `COUNT(*)`   | Number of rows in the group                              |
| `COUNT(x)`   | Number of rows where `x` is non-null                     |
| `SUM(x)`     | Sum of numeric values, ignoring nulls                    |
| `AVG(x)`     | Average of numeric values, ignoring nulls                |
| `MIN(x)`     | Smallest non-null value                                  |
| `MAX(x)`     | Largest non-null value                                   |

Aggregates may only appear in `SELECT` and `HAVING`. Using one in
`WHERE` is a query error.

## Data types and NULL

Values keep their SQL type through the evaluator:

- **Strings** parsed as ISO dates (`YYYY-MM-DD`) or RFC 3339 timestamps
  are treated as `date` / `datetime`, not strings. This is what lets
  `DATE_ADD`, `<`, and `BETWEEN` work on frontmatter that looks like
  text on disk.
- **Integers and floats** compare naturally; mixed arithmetic promotes
  integers to floats.
- **Lists** support equality and `LENGTH()`. Element-wise membership
  is not supported in this version — for tag queries, use the `--tag`
  flag on `note list` or `search`, or filter in a script.
- **NULL** propagates: `NULL = anything` is unknown, and unknown rows
  don't match `WHERE`. `ORDER BY` sorts NULLs first on `ASC`.

## LIKE patterns

`%` matches any sequence of characters; `_` matches a single character.
Escape either with `\` to match it literally:

```sql
SELECT title FROM note WHERE title LIKE 'Plant%'
SELECT title FROM note WHERE title LIKE 'P_ant%'   -- P, any char, 'ant...'
SELECT path  FROM note WHERE path  LIKE 'projects/%'
```

## Resource limits

Queries are bounded:

- Maximum **10,000** result rows per query
- Maximum **32** levels of expression nesting

A query that exceeds these limits returns a clean error rather than
running on.

## Examples

Active design docs, most recently updated first:

```sql
SELECT title, updated
FROM design-doc
WHERE status = 'active'
ORDER BY updated DESC
LIMIT 10
```

Notes that haven't been updated in a year:

```sql
SELECT title, updated
FROM note
WHERE updated < DATE_ADD(TODAY(), -365)
```

Notes with at least one tag:

```sql
SELECT title, tags
FROM note
WHERE LENGTH(tags) > 0
```

For "notes tagged X", the SQL engine doesn't yet support scalar-in-list
membership — use `muninn note list --tag X` or filter tags inside a
script instead.

See also: [Scripting](/docs/muninn/scripting/) for calling `query()`
from inside notes, [Types](/docs/muninn/types/) for defining the
fields you'll query against.
