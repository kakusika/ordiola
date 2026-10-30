import { closeBrackets, closeBracketsKeymap } from "@codemirror/autocomplete";
import { markdown } from "@codemirror/lang-markdown";
import { bracketMatching, foldGutter, foldKeymap, indentOnInput } from "@codemirror/language";
import { highlightSelectionMatches, searchKeymap } from "@codemirror/search";
import type { Extension } from "@codemirror/state";
import { EditorView, keymap } from "@codemirror/view";

/** Syntax of the source editor. A Tomet document is shown as Markdown for now. */
export function sourceLanguage(path: string): Extension {
  const ext = path.split(".").pop()?.toLowerCase();
  return ext === "tmt" || ext === "md" || ext === "markdown" ? markdown() : [];
}

/**
 * What the source editor has beyond the minimal editor of @telaion/editor: folding, bracket
 * matching and closing, search, and highlighting of the matches of the selection. The editor fills
 * its pane, so the ring that @telaion/editor draws around a focused editor is left out here; the
 * caret and the active line show where the focus is, as they always did.
 */
export const sourceExtras: Extension = [
  foldGutter(),
  indentOnInput(),
  bracketMatching(),
  closeBrackets(),
  highlightSelectionMatches(),
  keymap.of([...closeBracketsKeymap, ...searchKeymap, ...foldKeymap]),
  // Three classes, to beat the two of the ring in @telaion/editor.
  EditorView.theme({ "&.cm-editor.cm-focused": { outline: "none" } }),
];
