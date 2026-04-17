# Nótt & Dagr — Asgard EHS theme specification

Official syntax highlighting and design specification for the Asgard EHS theme
ecosystem. This document follows the structural conventions established by the
[Dracula Theme specification](https://draculatheme.com/spec) by Zeno Rocha and
Lucas de França; all color values, names, and design decisions are original to
the Asgard EHS project.

## Overview

Nótt and Dagr are paired dark/light color themes designed for code editors,
terminal emulators, and the Asgard EHS desktop applications (Muninn, Huginn,
Odin, Heimdall, Bifrost). They share a common role architecture so that
switching modes preserves semantic meaning — a keyword is the same _kind_ of
thing in both themes, even when the specific color shifts for the surface.

**Variants:**

- **Nótt** — Dark theme. Named for the Norse personification of Night.
- **Dagr** — Light theme. Named for the Norse personification of Day, son of
  Nótt.

**Design intent.** Nótt draws on aurora-over-night-sky: a cool petrol-slate
ground with desaturated accent hues sitting where vibrant gems would sit in a
gothic theme. Dagr inverts this as morning-light-on-parchment: a warm cream
ground with deepened, earthier counterparts of the same accent roles. The two
themes are meant to be read as a pair — Night and Day from the same myth, same
architecture, same semantic mapping.

## Color palette

### Nótt (dark)

| Token            | Hex       | RGB           | HSL            | Usage                        |
| ---------------- | --------- | ------------- | -------------- | ---------------------------- |
| **Background**   | `#1A1F2E` | 26, 31, 46    | 225°, 28%, 14% | Main editor background       |
| **Current line** | `#262C3F` | 38, 44, 63    | 226°, 25%, 20% | Active line highlight        |
| **Selection**    | `#3D4457` | 61, 68, 87    | 224°, 18%, 29% | Text selection               |
| **Foreground**   | `#E6E4DA` | 230, 228, 218 | 50°, 21%, 88%  | Default text                 |
| **Comment**      | `#757C90` | 117, 124, 144 | 225°, 11%, 51% | Comments, disabled code      |
| **Red**          | `#E35F5B` | 227, 95, 91   | 2°, 72%, 62%   | Errors, warnings, deletions  |
| **Orange**       | `#E8A25C` | 232, 162, 92  | 30°, 75%, 64%  | Numbers, constants, booleans |
| **Yellow**       | `#E6DC82` | 230, 220, 130 | 54°, 69%, 71%  | Strings, text content        |
| **Green**        | `#8DC776` | 141, 199, 118 | 103°, 42%, 62% | Functions, methods           |
| **Cyan**         | `#6EB8D6` | 110, 184, 214 | 197°, 56%, 64% | Classes, types, support      |
| **Purple**       | `#A692D6` | 166, 146, 214 | 258°, 47%, 71% | Instance reserved words      |
| **Pink**         | `#DE80A4` | 222, 128, 164 | 337°, 59%, 69% | Keywords, storage types      |

### Dagr (light)

| Token            | Hex       | RGB           | HSL            | Usage                        |
| ---------------- | --------- | ------------- | -------------- | ---------------------------- |
| **Background**   | `#F5F1E4` | 245, 241, 228 | 46°, 45%, 93%  | Main editor background       |
| **Current line** | `#EDE9DB` | 237, 233, 219 | 47°, 31%, 89%  | Active line highlight        |
| **Selection**    | `#C9D1DF` | 201, 209, 223 | 219°, 25%, 83% | Text selection               |
| **Foreground**   | `#1F2330` | 31, 35, 48    | 226°, 21%, 15% | Default text                 |
| **Comment**      | `#6B6A58` | 107, 106, 88  | 56°, 10%, 38%  | Comments, disabled code      |
| **Red**          | `#B53D3A` | 181, 61, 58   | 1°, 52%, 47%   | Errors, warnings, deletions  |
| **Orange**       | `#8F5C18` | 143, 92, 24   | 35°, 71%, 33%  | Numbers, constants, booleans |
| **Yellow**       | `#6E601A` | 110, 96, 26   | 50°, 62%, 27%  | Strings, text content        |
| **Green**        | `#3A6525` | 58, 101, 37   | 100°, 46%, 27% | Functions, methods           |
| **Cyan**         | `#2B6A8A` | 43, 106, 138  | 200°, 53%, 36% | Classes, types, support      |
| **Purple**       | `#5B4A9E` | 91, 74, 158   | 252°, 36%, 45% | Instance reserved words      |
| **Pink**         | `#A84272` | 168, 66, 114  | 333°, 44%, 46% | Keywords, storage types      |

## ANSI color palette

For terminal applications. Following the convention where `AnsiBlue` maps to the
palette's Purple and `AnsiMagenta` maps to the palette's Pink — this keeps
terminal output (directory listings, diff hunks, prompt segments) harmonized
with editor syntax highlighting.

### Nótt

| ANSI token            | Hex       | RGB           |
| --------------------- | --------- | ------------- |
| **AnsiBlack**         | `#13171F` | 19, 23, 31    |
| **AnsiRed**           | `#E35F5B` | 227, 95, 91   |
| **AnsiGreen**         | `#8DC776` | 141, 199, 118 |
| **AnsiYellow**        | `#E6DC82` | 230, 220, 130 |
| **AnsiBlue**          | `#A692D6` | 166, 146, 214 |
| **AnsiMagenta**       | `#DE80A4` | 222, 128, 164 |
| **AnsiCyan**          | `#6EB8D6` | 110, 184, 214 |
| **AnsiWhite**         | `#E6E4DA` | 230, 228, 218 |
| **AnsiBrightBlack**   | `#757C90` | 117, 124, 144 |
| **AnsiBrightRed**     | `#EC7A77` | 236, 122, 119 |
| **AnsiBrightGreen**   | `#A5D690` | 165, 214, 144 |
| **AnsiBrightYellow**  | `#EFE69D` | 239, 230, 157 |
| **AnsiBrightBlue**    | `#C0B1E6` | 192, 177, 230 |
| **AnsiBrightMagenta** | `#E89AB7` | 232, 154, 183 |
| **AnsiBrightCyan**    | `#95CCE3` | 149, 204, 227 |
| **AnsiBrightWhite**   | `#F5F3EC` | 245, 243, 236 |

### Dagr

| ANSI token            | Hex       | RGB           |
| --------------------- | --------- | ------------- |
| **AnsiBlack**         | `#F5F1E4` | 245, 241, 228 |
| **AnsiRed**           | `#B53D3A` | 181, 61, 58   |
| **AnsiGreen**         | `#3A6525` | 58, 101, 37   |
| **AnsiYellow**        | `#6E601A` | 110, 96, 26   |
| **AnsiBlue**          | `#5B4A9E` | 91, 74, 158   |
| **AnsiMagenta**       | `#A84272` | 168, 66, 114  |
| **AnsiCyan**          | `#2B6A8A` | 43, 106, 138  |
| **AnsiWhite**         | `#1F2330` | 31, 35, 48    |
| **AnsiBrightBlack**   | `#6B6A58` | 107, 106, 88  |
| **AnsiBrightRed**     | `#C04E4B` | 192, 78, 75   |
| **AnsiBrightGreen**   | `#477A2F` | 71, 122, 47   |
| **AnsiBrightYellow**  | `#847224` | 132, 114, 36  |
| **AnsiBrightBlue**    | `#6E5BBC` | 110, 91, 188  |
| **AnsiBrightMagenta** | `#C14F86` | 193, 79, 134  |
| **AnsiBrightCyan**    | `#3680A5` | 54, 128, 165  |
| **AnsiBrightWhite**   | `#2A2D3B` | 42, 45, 59    |

## UI color palette

Surface layers beyond the editor — application chrome, panels, dialogs,
popovers.

### Nótt

| Context                           | Hex       | RGB        | HSL            |
| --------------------------------- | --------- | ---------- | -------------- |
| **Floating interactive elements** | `#262C3F` | 38, 44, 63 | 226°, 25%, 20% |
| **Background lighter**            | `#30374A` | 48, 55, 74 | 224°, 21%, 24% |
| **Background light**              | `#242A3B` | 36, 42, 59 | 224°, 24%, 19% |
| **Background dark**               | `#13171F` | 19, 23, 31 | 220°, 24%, 10% |
| **Background darker**             | `#0C0F16` | 12, 15, 22 | 222°, 29%, 7%  |

### Dagr

| Context                           | Hex       | RGB           | HSL           |
| --------------------------------- | --------- | ------------- | ------------- |
| **Floating interactive elements** | `#EDE9DB` | 237, 233, 219 | 47°, 31%, 89% |
| **Background lighter**            | `#FBF8EC` | 251, 248, 236 | 47°, 60%, 95% |
| **Background light**              | `#E6E1D1` | 230, 225, 209 | 46°, 30%, 86% |
| **Background dark**               | `#D2CEC0` | 210, 206, 192 | 47°, 17%, 79% |
| **Background darker**             | `#BCB8AB` | 188, 184, 171 | 46°, 12%, 70% |

### Functional colors

UI-specific colors for interactive elements, borders, and status indicators.
**Do not use in editor or terminal applications** — these are tuned for the app
chrome surfaces and carry stronger saturation than the editor accents would
tolerate on a code background.

| Token                 | Nótt      | Dagr      | Usage                                |
| --------------------- | --------- | --------- | ------------------------------------ |
| **Functional Red**    | `#C74A35` | `#A0351F` | Destructive actions, critical alerts |
| **Functional Orange** | `#A88714` | `#8C6E0E` | Warnings, cautions                   |
| **Functional Green**  | `#3E8855` | `#2A6E3F` | Success states, confirmations        |
| **Functional Cyan**   | `#2E7AB5` | `#1A5B8F` | Information, links                   |
| **Functional Purple** | `#6B52B8` | `#4F3B95` | Focus indicators                     |

## Syntax highlighting rules

### Token classification

Following TextMate scoping conventions for consistent highlighting across
editors.

#### Primary tokens

**Keywords and storage** → `Pink`

- Language keywords: `if`, `else`, `return`, `fn`, `struct`, `impl`, `match`
- Storage modifiers: `static`, `pub`, `const`, `let`, `mut`, `ref`
- Control flow: `loop`, `while`, `break`, `continue`, `await`

**Functions and methods** → `Green`

- Function declarations and calls
- Method invocations
- Built-in functions and macros

**Classes and types** → `Cyan`

- Type names and constructors (`String`, `Vec`, `BatchRecord`)
- Primitive type annotations (`u64`, `f64`, `bool`)
- Traits, enums, interfaces
- Generic type parameters

**Strings and text** → `Yellow`

- String literals (`"hello"`, raw strings `r#"..."#`, byte strings `b"..."`)
- Markup text content
- Attribute values in HTML/XML
- Escape sequences

**Numbers and constants** → `Orange`

- Numeric literals (`42`, `3.14`, `0xFF`, `1e5`, `1_000_000`)
- Boolean values (`true`, `false`)
- Language constants (`None`, `null`, `undefined`, `NaN`)

**Comments** → `Comment`

- Single-line: `//`, `#`, `--`
- Multi-line: `/* */`, `<!-- -->`
- Documentation blocks: `///`, `//!`, `/** */`
- Annotations and decorators

**Support and built-ins** → `Cyan`

- Standard library items
- Regular expressions
- CSS properties and units
- HTML/XML tag and attribute names

**Variables and identifiers** → `Foreground`

- Variable names and parameters
- Struct fields and object properties
- Default text content

**Instance reserved words** → `Purple` _italic_

- Words that reference the current instance: `this`, `self`, `super`, `Self`
- Rendered in Purple with italic styling across all languages for consistency

**Errors and warnings** → `Red`

- Syntax errors
- Deprecated code
- Invalid tokens
- Diff deletions

### Styling modifiers

- **Italic:** comments, type parameters, documentation, instance reserved words
- **Bold:** strong emphasis (use sparingly)
- **Underline:** hyperlinks, misspelled words

### Special rules

1. **Braces and parentheses** should match the foreground color of the currently
   scoped position — Purple for headings, Foreground for regular text.
2. **Instance reserved words** (`self`, `this`, `super`, `Self`) are always
   rendered in Purple with italic styling, across every language.
3. **Generic type parameters** use Orange italic.

## Test snippet

Rust reference implementation. A valid Nótt or Dagr port renders this with every
token category correctly colored.

```rust
// Nótt & Dagr: compliance memory capture
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct BatchRecord {
    batch_number: String,
    quantity_kg: f64,
    captured_at: u64,
}

impl BatchRecord {
    fn new(batch_number: String, quantity_kg: f64) -> Self {
        Self {
            batch_number,
            quantity_kg,
            captured_at: 1713312000,
        }
    }

    fn slug(&self) -> String {
        self.batch_number.to_lowercase()
    }
}

fn main() {
    let record = BatchRecord::new(
        "CHEM-2026-0417".to_string(),
        42.5,
    );
    println!("captured: {} ({} kg)", record.slug(), record.quantity_kg);
}
```

Validation checklist when reviewing an implementation:

- `//` comment → Comment color, italic
- `use`, `struct`, `impl`, `fn`, `let` → Pink
- `serde`, `Deserialize`, `Serialize`, `String`, `u64`, `f64`, `BatchRecord` →
  Cyan
- `derive`, `new`, `slug`, `to_string`, `to_lowercase`, `main`, `println!` →
  Green
- `"CHEM-2026-0417"`, `"captured: {} ({} kg)"` → Yellow
- `1713312000`, `42.5` → Orange
- `Self` (type position), `self` (value position) → Purple italic
- `batch_number`, `quantity_kg`, `captured_at`, `record` → Foreground

## Implementation guidelines

### Accessibility

- Maintain WCAG 2.1 Level AA contrast (4.5:1 for body text) between foreground
  and background on both themes.
- Comment color sits at ~3.9:1 on Nótt and ~4.8:1 on Dagr. Comments are
  intentionally de-emphasized; implementations targeting strict AA compliance
  throughout may use the `AnsiBrightBlack` value (`#878EA1` on Nótt, `#5A594A`
  on Dagr) as an alternate comment color.
- Do not rely on color alone to convey state; pair with an icon, typographic
  weight, or text label.
- Validate under common color-vision deficiencies (protanopia, deuteranopia,
  tritanopia).

### Consistency

1. **Priority order.** Follow the token classification hierarchy — a
   more-specific scope wins over a less-specific one.
2. **Fallback handling.** Unrecognized tokens render in Foreground.
3. **Semantic consistency.** Same meaning = same color across languages. A
   keyword is Pink in Rust, Python, and TypeScript alike.
4. **Cross-theme parity.** If `self` is Purple italic in Nótt, it is Purple
   italic in Dagr. The role mapping is identical; only the surface-adapted
   values differ.

### UI component guidelines

**Borders and separators**

- Subtle borders: Current Line color
- Interactive borders: corresponding Functional color
- Focus rings: Functional Purple

**State indicators**

- Success → Functional Green
- Warning → Functional Orange
- Error → Functional Red
- Info → Functional Cyan

**Shadows and depth**

- Prefer surface-based elevation (the Floating interactive elements layer) over
  drop-shadows.
- When a shadow is unavoidable, use `rgba(0, 0, 0, 0.15)` on Nótt and
  `rgba(12, 15, 22, 0.08)` on Dagr.
- Shadow color should harmonize with the surface beneath; avoid shadows darker
  than the darkest background layer.

**Visual hierarchy**

- **High priority** (interactive elements, errors, primary actions) →
  full-saturation accent or Functional color
- **Medium priority** (navigation, labels, secondary content) → Foreground or
  Comment
- **Low priority** (decorative dividers, background elements) → Current Line or
  Background light/lighter

## Attribution

The role architecture, token classification, and specification structure in this
document follow conventions established by the Dracula Theme specification by
Zeno Rocha and Lucas de França, available at
[draculatheme.com/spec](https://draculatheme.com/spec). The Dracula spec is a
functional framework for describing syntax-highlighting themes; this document
reuses that framework.

All color values, theme names (Nótt, Dagr), design decisions, and accompanying
assets in this specification are original to the Asgard EHS project and are not
derived from Dracula's palette.

---

_Part of the Asgard EHS ecosystem. Maintained alongside Muninn, Huginn, Odin,
Heimdall, and Bifrost._
