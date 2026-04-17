---
layout: default
title: Types
permalink: /docs/muninn/types/
---

# Types

Muninn uses [mdbase](https://github.com/asgardehs/mdbase) for typed
frontmatter. A type is a schema that declares what fields a note can
have, which ones are required, what they contain, and which notes the
schema applies to.

Typing is optional — you can have a vault full of notes with no types
at all. But once you start asking structured questions ("what's
active?", "what's due this week?"), types keep your frontmatter
consistent so queries and scripts work reliably.

## Where types live

Type definitions are markdown files in `.muninn/types/`:

```
.muninn/
└── types/
    ├── note.md
    ├── journal.md
    └── project.md
```

Each file is a YAML frontmatter block describing the schema, with an
optional markdown body for documentation. `muninn init` creates a
starter `note.md` for you.

## File format

A minimal type:

```yaml
---
name: note
description: A basic note
fields:
  title:
    type: string
    required: true
  tags:
    type: list
    items:
      type: string
---
The default note type. All notes should have a title.
```

A type with inheritance and matching rules:

```yaml
---
name: journal
description: Daily journal entry
extends: note
fields:
  date:
    type: date
    required: true
  mood:
    type: string
match:
  path_glob: "journal/*.md"
---
A daily journal entry. Inherits title and tags from `note`.
```

## Top-level type fields

| Field           | Type              | Purpose                                                         |
| --------------- | ----------------- | --------------------------------------------------------------- |
| `name`          | string            | Type identifier, used in `type:` frontmatter and `FROM` clauses |
| `description`   | string            | One-line summary, shown in `type list`                          |
| `extends`       | string            | Inherit fields from another type                                |
| `fields`        | map               | Field definitions (see below)                                   |
| `match`         | map               | Rules for matching notes that don't declare `type:` explicitly  |
| `path_pattern`  | string            | Suggested filename pattern (advisory; used by future tooling)   |
| `strict`        | `forbid` / `warn` | How to treat frontmatter keys not declared in `fields`          |

## Field types

| Type       | Values                                        |
| ---------- | --------------------------------------------- |
| `string`   | Free-form text                                |
| `integer`  | Whole numbers                                 |
| `number`   | Integers or decimals                          |
| `boolean`  | `true` / `false`                              |
| `date`     | `YYYY-MM-DD`                                  |
| `datetime` | RFC 3339 timestamp (`2026-04-16T12:00:00Z`)   |
| `time`     | `HH:MM:SS`                                    |
| `enum`     | One of a named list of values                 |
| `list`     | Ordered collection of values of one type      |
| `object`   | Nested map of named sub-fields                |
| `link`     | Wikilink target (a note name)                 |
| `any`      | Anything — no validation                      |

## Field constraints

Additional keys on a field definition apply constraints beyond type:

| Constraint               | Applies to                  | Purpose                                |
| ------------------------ | --------------------------- | -------------------------------------- |
| `required`               | any                         | Validation fails if the field is absent |
| `default`                | any                         | Value assigned on note creation        |
| `description`            | any                         | Help text, shown in `type show`        |
| `deprecated`             | any                         | Flag the field for removal             |
| `unique`                 | any                         | Value must be unique across notes      |
| `minLength` / `maxLength` | `string`                   | Length bounds                          |
| `pattern`                | `string`                    | Regex the value must match             |
| `min` / `max`            | `integer`, `number`, `date`, `datetime` | Value bounds               |
| `values`                 | `enum`                      | The list of allowed values             |
| `items`                  | `list`                      | Schema for each list element           |
| `minItems` / `maxItems`  | `list`                      | Collection size bounds                 |
| `fields`                 | `object`                    | Nested field definitions               |
| `target`                 | `link`                      | Type(s) the link should point to       |
| `validate_exists`        | `link`                      | Require the target note to exist       |

## Generated fields

A field with a `generated:` strategy is filled in automatically by
`muninn note new`:

| Strategy       | Behavior                                                   |
| -------------- | ---------------------------------------------------------- |
| `uuid`         | A random UUID, set once on creation                        |
| `uuid_short`   | The first 8 characters of a UUID                           |
| `slug`         | Slugified form of the `title`                              |
| `now`          | RFC 3339 timestamp at creation                             |
| `now_on_write` | RFC 3339 timestamp, updated every write                    |

```yaml
fields:
  id:
    type: string
    generated: uuid
  created:
    type: datetime
    generated: now
  updated:
    type: datetime
    generated: now_on_write
```

## Inheritance

A type can `extends` another type. The child inherits every field from
the parent; adding a field with the same name overrides the parent's
definition.

```yaml
---
name: journal
extends: note
fields:
  date:
    type: date
    required: true
---
```

Inheritance is single — a type has at most one parent. Cycles are
rejected at load time.

## Type matching

How does Muninn decide which type a given note is?

1. **Explicit** — if the note's frontmatter has `type: journal` (or
   whichever keys are listed in `settings.explicit_type_keys`), that
   type wins.
2. **Match rules** — otherwise, every type with a `match:` block is
   tested in turn. A match requires every condition in the block to
   hold.

Supported match conditions:

| Condition         | Example                                      | Meaning                                          |
| ----------------- | -------------------------------------------- | ------------------------------------------------ |
| `path_glob`       | `"journal/*.md"`                             | File path matches the glob                       |
| `fields_present`  | `["mood", "weather"]`                        | All named fields exist in frontmatter            |
| `where`           | `status: { eq: active }`                     | Frontmatter field matches a predicate           |

The `where` predicate supports `eq`, `ne`, `contains`, `starts_with`,
and `in`:

```yaml
match:
  where:
    status:
      in: [active, pending]
    area:
      eq: work
```

## Strict mode

By default, notes can include frontmatter keys not declared in the
type — they're just ignored by validation. Set `strict:` to tighten
that:

- `forbid` — unknown keys are a validation **error**
- `warn` — unknown keys are a validation **warning**

```yaml
---
name: incident
strict: forbid
fields:
  status:
    type: enum
    values: [active, resolved]
---
```

## Commands

### type list

Show every defined type with field counts:

```bash
muninn type list
```

```
+---------+---------------------+---------+--------+
| Name    | Description         | Extends | Fields |
+---------+---------------------+---------+--------+
| folder  | Folder metadata     |         | 2      |
+---------+---------------------+---------+--------+
| journal | Daily journal entry | note    | 5      |
+---------+---------------------+---------+--------+
| note    | A basic note        |         | 3      |
+---------+---------------------+---------+--------+
```

### type show

Show a single type's fields (resolved through inheritance):

```bash
muninn type show journal
```

```
journal
  Daily journal entry
  extends: note

  Fields:
    title (string, required)
    tags (list)
    status (enum, values=["active", "done", "archived"])
    date (date, required)
    mood (string)
```

### validate

Check notes against their matched types:

```bash
muninn validate                        # all notes
muninn validate projects/plant-ops.md  # a single note
```

Errors exit with a non-zero code; warnings don't. Add `--json` for
structured output with `path`, `field`, `code`, `message`, and
`severity`.

## Error codes

Validation errors come with short stable codes you can match on:

| Code                    | Meaning                                           |
| ----------------------- | ------------------------------------------------- |
| `missing_required`      | A required field isn't present                    |
| `type_mismatch`         | A field's value doesn't match its declared type   |
| `constraint_violation`  | A value violates `min`, `max`, `pattern`, etc.    |
| `enum_not_allowed`      | An enum field has a value outside its `values`    |
| `link_broken`           | A `link` field points at a missing note (with `validate_exists`) |
| `unknown_field`         | An undeclared key in a `strict` type              |
| `duplicate_value`       | A `unique` field's value appears elsewhere        |

See also: [Query](/docs/muninn/query/) for querying typed fields with
SQL, [Notes](/docs/muninn/notes/) for the baseline frontmatter
conventions.
