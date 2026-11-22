import { markdown } from "@codemirror/lang-markdown";
import type { Extension } from "@codemirror/state";

/** Syntax of the source editor. A Tomet document is shown as Markdown for now. */
export function sourceLanguage(path: string): Extension {
  const ext = path.split(".").pop()?.toLowerCase();
  return ext === "tmt" || ext === "md" || ext === "markdown" || ext === "typ"
    ? markdown()
    : [];
}
