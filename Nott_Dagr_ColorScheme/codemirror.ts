/**
 * Nótt & Dagr CodeMirror 6 themes for Asgard EHS.
 *
 * Each export is an Extension bundle combining:
 *   - editor chrome (background, gutter, selection, cursor, tooltips)
 *   - syntax highlighting (via Lezer tags → spec roles)
 *
 * Usage:
 *   import { EditorView, basicSetup } from "codemirror";
 *   import { rust } from "@codemirror/lang-rust";
 *   import { nott } from "./theme";
 *
 *   new EditorView({
 *     parent,
 *     extensions: [basicSetup, rust(), nott],
 *   });
 */

import { EditorView } from "@codemirror/view";
import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { tags as t } from "@lezer/highlight";
import type { Extension } from "@codemirror/state";

// --- Palette (mirrors theme.css; duplicated because CodeMirror does not
//     read from CSS custom properties) ---

interface ThemePalette {
  bg: string;
  fg: string;
  comment: string;
  selection: string;
  currentLine: string;
  floating: string;
  red: string;
  orange: string;
  yellow: string;
  green: string;
  cyan: string;
  purple: string;
  pink: string;
}

const nottPalette: ThemePalette = {
  bg:          "#1A1F2E",
  fg:          "#E6E4DA",
  comment:     "#757C90",
  selection:   "#3D4457",
  currentLine: "#262C3F",
  floating:    "#262C3F",
  red:         "#E35F5B",
  orange:      "#E8A25C",
  yellow:      "#E6DC82",
  green:       "#8DC776",
  cyan:        "#6EB8D6",
  purple:      "#A692D6",
  pink:        "#DE80A4",
};

const dagrPalette: ThemePalette = {
  bg:          "#F5F1E4",
  fg:          "#1F2330",
  comment:     "#6B6A58",
  selection:   "#C9D1DF",
  currentLine: "#EDE9DB",
  floating:    "#EDE9DB",
  red:         "#B53D3A",
  orange:      "#8F5C18",
  yellow:      "#6E601A",
  green:       "#3A6525",
  cyan:        "#2B6A8A",
  purple:      "#5B4A9E",
  pink:        "#A84272",
};

// --- Theme builder ---

function makeEditorTheme(c: ThemePalette, dark: boolean) {
  return EditorView.theme(
    {
      "&": {
        color: c.fg,
        backgroundColor: c.bg,
      },
      ".cm-content": {
        caretColor: c.fg,
      },
      ".cm-cursor, .cm-dropCursor": {
        borderLeftColor: c.fg,
      },
      "&.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection":
        {
          backgroundColor: c.selection,
        },
      ".cm-activeLine": {
        backgroundColor: c.currentLine,
      },
      ".cm-gutters": {
        backgroundColor: c.bg,
        color: c.comment,
        border: "none",
      },
      ".cm-activeLineGutter": {
        backgroundColor: c.currentLine,
      },
      ".cm-foldPlaceholder": {
        backgroundColor: "transparent",
        border: "none",
        color: c.purple,
      },
      ".cm-tooltip": {
        backgroundColor: c.floating,
        border: `1px solid ${c.selection}`,
        color: c.fg,
      },
      ".cm-tooltip .cm-tooltip-arrow:before": {
        borderTopColor: "transparent",
        borderBottomColor: "transparent",
      },
      ".cm-tooltip .cm-tooltip-arrow:after": {
        borderTopColor: c.floating,
        borderBottomColor: c.floating,
      },
      ".cm-tooltip-autocomplete > ul > li[aria-selected]": {
        backgroundColor: c.selection,
        color: c.fg,
      },
      ".cm-searchMatch": {
        backgroundColor: c.orange + "33",
        outline: `1px solid ${c.orange}`,
      },
      ".cm-searchMatch.cm-searchMatch-selected": {
        backgroundColor: c.orange + "66",
      },
      ".cm-panels": {
        backgroundColor: c.floating,
        color: c.fg,
      },
      ".cm-panels-top": { borderBottom: `1px solid ${c.selection}` },
      ".cm-panels-bottom": { borderTop: `1px solid ${c.selection}` },
    },
    { dark },
  );
}

function makeHighlightStyle(c: ThemePalette) {
  return HighlightStyle.define([
    // Comments → comment color, italic
    {
      tag: [t.comment, t.lineComment, t.blockComment, t.docComment],
      color: c.comment,
      fontStyle: "italic",
    },

    // Keywords & storage → Pink
    {
      tag: [
        t.keyword,
        t.controlKeyword,
        t.moduleKeyword,
        t.operatorKeyword,
        t.definitionKeyword,
        t.modifier,
      ],
      color: c.pink,
    },

    // Strings → Yellow
    {
      tag: [t.string, t.special(t.string), t.regexp, t.escape],
      color: c.yellow,
    },

    // Numbers & constants → Orange
    {
      tag: [t.number, t.integer, t.float, t.bool, t.null, t.atom],
      color: c.orange,
    },

    // Functions & methods → Green
    {
      tag: [
        t.function(t.variableName),
        t.function(t.propertyName),
        t.macroName,
      ],
      color: c.green,
    },

    // Classes, types, built-ins → Cyan
    {
      tag: [
        t.typeName,
        t.className,
        t.namespace,
        t.definition(t.typeName),
        t.standard(t.variableName),
        t.standard(t.propertyName),
        t.tagName,
        t.attributeName,
      ],
      color: c.cyan,
    },

    // Instance reserved words (self, this, super, Self) → Purple italic
    {
      tag: t.self,
      color: c.purple,
      fontStyle: "italic",
    },

    // Generic type parameters → Orange italic (per spec rule 3)
    {
      tag: t.typeOperator,
      color: c.orange,
      fontStyle: "italic",
    },

    // Errors & deletions → Red
    { tag: [t.invalid, t.deleted], color: c.red },

    // Diff markers
    { tag: t.inserted, color: c.green },
    { tag: t.changed, color: c.orange },

    // Markup (markdown, etc.)
    { tag: t.heading, color: c.purple, fontWeight: "bold" },
    { tag: t.emphasis, fontStyle: "italic" },
    { tag: t.strong, fontWeight: "bold" },
    { tag: t.strikethrough, textDecoration: "line-through" },
    { tag: [t.link, t.url], color: c.cyan, textDecoration: "underline" },
    { tag: t.quote, color: c.comment, fontStyle: "italic" },

    // Variables, properties, operators, punctuation fall through to foreground
    // (CodeMirror uses the editor's default color when no rule matches)
  ]);
}

function makeTheme(palette: ThemePalette, dark: boolean): Extension {
  return [makeEditorTheme(palette, dark), syntaxHighlighting(makeHighlightStyle(palette))];
}

// --- Exports ---

export const nott: Extension = makeTheme(nottPalette, true);
export const dagr: Extension = makeTheme(dagrPalette, false);

/** Pick the right theme by name — handy when wiring up from React state. */
export function themeByName(name: "nott" | "dagr"): Extension {
  return name === "nott" ? nott : dagr;
}
