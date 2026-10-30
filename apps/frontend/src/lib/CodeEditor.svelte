<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { basicSetup } from "codemirror";
  import { EditorView, keymap } from "@codemirror/view";
  import { EditorState, Compartment } from "@codemirror/state";
  import { markdown } from "@codemirror/lang-markdown";
  import { oneDark } from "@codemirror/theme-one-dark";

  interface Props {
    content: string;
    filePath?: string;
    onChange?: (newContent: string) => void;
    onSave?: () => void;
  }

  let {
    content,
    filePath = "",
    onChange,
    onSave,
  }: Props = $props();

  let editorContainer = $state<HTMLDivElement | null>(null);
  let view: EditorView | null = null;
  const languageCompartment = new Compartment();

  // Custom theme tailored for Ordiola's stone palette
  const ordiolaEditorTheme = EditorView.theme({
    "&": {
      height: "100%",
      fontSize: "14px",
      backgroundColor: "transparent",
      color: "#e7e5e4", // stone-200
      fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace',
    },
    ".cm-content": {
      caretColor: "#38bdf8", // sky-400
      padding: "16px 0",
      lineHeight: "1.65",
    },
    ".cm-cursor, .cm-dropCursor": {
      borderLeftColor: "#38bdf8",
    },
    "&.cm-focused .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection": {
      backgroundColor: "#292524 !important", // stone-800
    },
    ".cm-gutters": {
      backgroundColor: "#0c0a09", // stone-950
      color: "#78716c", // stone-500
      borderRight: "1px solid #1c1917", // stone-900
      paddingRight: "8px",
    },
    ".cm-activeLine": {
      backgroundColor: "#1c191780", // stone-900 with alpha
    },
    ".cm-activeLineGutter": {
      backgroundColor: "#1c1917",
      color: "#f5f5f4",
    },
    ".cm-scroller": {
      overflow: "auto",
      fontFamily: "inherit",
    },
  }, { dark: true });

  function getLanguageExtension(path: string) {
    const ext = path.split(".").pop()?.toLowerCase();
    if (ext === "tmt" || ext === "md" || ext === "markdown") {
      return markdown();
    }
    return [];
  }

  onMount(() => {
    if (!editorContainer) return;

    const saveKeymap = keymap.of([
      {
        key: "Mod-s",
        run: () => {
          onSave?.();
          return true;
        },
      },
    ]);

    const updateListener = EditorView.updateListener.of((update) => {
      if (update.docChanged) {
        const val = update.state.doc.toString();
        onChange?.(val);
      }
    });

    const startState = EditorState.create({
      doc: content,
      extensions: [
        basicSetup,
        ordiolaEditorTheme,
        oneDark,
        languageCompartment.of(getLanguageExtension(filePath)),
        saveKeymap,
        updateListener,
        EditorView.lineWrapping,
      ],
    });

    view = new EditorView({
      state: startState,
      parent: editorContainer,
    });
  });

  onDestroy(() => {
    view?.destroy();
  });

  // Keep editor content in sync when file changes externally
  $effect(() => {
    if (view) {
      const currentDoc = view.state.doc.toString();
      if (content !== currentDoc) {
        view.dispatch({
          changes: { from: 0, to: currentDoc.length, insert: content },
        });
      }
    }
  });

  // Update language when file path changes
  $effect(() => {
    if (view && filePath) {
      view.dispatch({
        effects: languageCompartment.reconfigure(getLanguageExtension(filePath)),
      });
    }
  });
</script>

<div class="h-full w-full overflow-hidden flex flex-col bg-stone-950 select-text">
  <div bind:this={editorContainer} class="h-full w-full overflow-hidden"></div>
</div>

<style>
  :global(.cm-editor) {
    height: 100%;
    outline: none !important;
  }
</style>
