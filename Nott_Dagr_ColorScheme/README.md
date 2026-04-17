# Nótt & Dagr — theme module

Drop-in theme system for Muninn's Tauri v2 + React + CodeMirror 6 stack. Implements the [Nótt & Dagr specification](../nott-dagr-spec.md) for this runtime.

## Dependencies

CodeMirror 6 core (almost certainly already installed if the editor is wired up):

```bash
npm install @codemirror/view @codemirror/state @codemirror/language @lezer/highlight
```

For Rust syntax highlighting:

```bash
npm install @codemirror/lang-rust
```

## File layout

```
src/theme/
├── theme.css         CSS custom properties — UI chrome
├── codemirror.ts     Extension bundles — editor
├── useTheme.ts       React hook — state + persistence
└── index.ts          Barrel
```

## Wiring (three steps)

### 1. Import the CSS once, at app entry

`src/main.tsx`:

```ts
import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./theme/theme.css";

ReactDOM.createRoot(document.getElementById("root")!).render(<App />);
```

### 2. Call the hook near the top of your component tree

`src/App.tsx`:

```tsx
import { useTheme } from "./theme";
import { Editor } from "./Editor";

export default function App() {
  const { theme, toggle } = useTheme();

  return (
    <div className="app">
      <header>
        <button onClick={toggle}>
          Switch to {theme === "nott" ? "Dagr" : "Nótt"}
        </button>
      </header>
      <Editor theme={theme} />
    </div>
  );
}
```

The hook sets `data-theme` on `<html>`, so `theme.css` takes effect immediately and every component downstream sees the right custom properties via `var(--color-bg)` etc.

### 3. Pass the theme to CodeMirror

`src/Editor.tsx`:

```tsx
import { useEffect, useRef } from "react";
import { EditorState } from "@codemirror/state";
import { EditorView, basicSetup } from "codemirror";
import { rust } from "@codemirror/lang-rust";
import { themeByName, type Theme } from "./theme";

interface EditorProps {
  theme: Theme;
  doc: string;
  onChange?: (value: string) => void;
}

export function Editor({ theme, doc, onChange }: EditorProps) {
  const parent = useRef<HTMLDivElement>(null);
  const view = useRef<EditorView | null>(null);

  useEffect(() => {
    if (!parent.current) return;

    view.current = new EditorView({
      state: EditorState.create({
        doc,
        extensions: [
          basicSetup,
          rust(),
          themeByName(theme),
          EditorView.updateListener.of((u) => {
            if (u.docChanged && onChange) {
              onChange(u.state.doc.toString());
            }
          }),
        ],
      }),
      parent: parent.current,
    });

    return () => view.current?.destroy();
  }, [theme]); // rebuild on theme switch; see note below

  return <div ref={parent} />;
}
```

## Using the palette in React components

Reference the CSS vars anywhere in your styles:

```tsx
function Sidebar() {
  return (
    <aside style={{
      background: "var(--color-bg-light)",
      borderRight: "1px solid var(--color-selection)",
      color: "var(--color-fg)",
    }}>
      ...
    </aside>
  );
}
```

Or if you're using Tailwind, extend `tailwind.config.js`:

```js
export default {
  theme: {
    extend: {
      colors: {
        bg: "var(--color-bg)",
        fg: "var(--color-fg)",
        comment: "var(--color-comment)",
        accent: {
          red: "var(--color-red)",
          orange: "var(--color-orange)",
          yellow: "var(--color-yellow)",
          green: "var(--color-green)",
          cyan: "var(--color-cyan)",
          purple: "var(--color-purple)",
          pink: "var(--color-pink)",
        },
      },
    },
  },
};
```

Now `bg-bg`, `text-accent-cyan`, etc. work and auto-adapt.

## Design notes

**Why two definitions of the palette?** The CSS vars drive your React UI chrome. CodeMirror's `EditorView.theme()` does not read CSS custom properties — it needs concrete string values at `Extension` construction time. The TS palette in `codemirror.ts` duplicates the hex values deliberately. If you change a color in one file, change it in the other.

**Why rebuild the editor on theme switch?** CodeMirror extensions are set at `EditorState` creation. You can swap them live via `StateEffect.reconfigure`, but the simpler approach for a full-theme swap is just tearing down and recreating — it's fast, and theme switches are rare user actions. If you're building a live-preview that switches often, graduate to the reconfigure approach.

**What's not styled.** The CodeMirror theme covers the standard editor chrome (gutter, tooltip, autocomplete, search, panels). If you add extensions with their own UI — diff viewers, minimap, collab cursors — they may need additional theme rules targeting their specific classes.

**Tauri v2 window chrome.** If you're using a custom title bar (`decorations: false` in `tauri.conf.json`), style it with the same CSS vars so it matches. The system title bar inherits from `color-scheme`, which `theme.css` sets correctly.
