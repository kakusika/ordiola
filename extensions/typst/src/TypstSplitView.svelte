<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { Alert } from "@telaion/primitives";
  import { SplitPane } from "@telaion/shell";
  import { CodeEditor } from "@telaion/editor";
  import TypstPreview from "./TypstPreview.svelte";
  import { compileTypst, exportTypstPdf } from "./api";
  import type { TypstCompileOutput, TypstDiagnostic } from "./types";

  interface Props {
    content: string;
    path: string;
    language?: any;
    extensions?: any;
    onChange?: (val: string) => void;
    onSave?: () => void;
  }

  let {
    content,
    path,
    language = [],
    extensions = [],
    onChange,
    onSave,
  }: Props = $props();

  let pages = $state<string[]>([]);
  let diagnostics = $state<TypstDiagnostic[]>([]);
  let isCompiling = $state<boolean>(false);
  let debounceTimer: any = null;
  // The outcome of the last PDF export, until it is dismissed.
  let notice = $state<{ tone: "ok" | "danger"; text: string } | null>(null);

  async function triggerCompile(text: string) {
    isCompiling = true;
    try {
      const res: TypstCompileOutput = await compileTypst(text, path);
      if (res.success) {
        pages = res.pages;
      }
      diagnostics = res.diagnostics;
    } catch (e: any) {
      diagnostics = [
        {
          severity: "error",
          message: String(e?.message || e),
          line: 1,
          column: 1,
          end_line: 1,
          end_column: 1,
        },
      ];
    } finally {
      isCompiling = false;
    }
  }

  function handleEditorChange(newVal: string) {
    onChange?.(newVal);
    if (debounceTimer) {
      clearTimeout(debounceTimer);
    }
    debounceTimer = setTimeout(() => {
      triggerCompile(newVal);
    }, 150);
  }

  async function handleExportPdf() {
    try {
      const defaultOut = path.replace(/\.typ$/, ".pdf");
      await exportTypstPdf(content, path, defaultOut);
      notice = { tone: "ok", text: `PDF を書き出しました: ${defaultOut}` };
    } catch (e: any) {
      notice = { tone: "danger", text: `PDF 書き出しエラー: ${e?.message || e}` };
    }
  }

  onMount(() => {
    triggerCompile(content);
  });

  onDestroy(() => {
    if (debounceTimer) {
      clearTimeout(debounceTimer);
    }
  });

  // Watch external content changes (e.g. tab switch)
  $effect(() => {
    if (content) {
      triggerCompile(content);
    }
  });
</script>

<div class="h-full w-full overflow-hidden flex flex-col bg-tln-canvas">
  {#if notice}
    <div class="p-2 shrink-0">
      <Alert tone={notice.tone} onDismiss={() => (notice = null)}>{notice.text}</Alert>
    </div>
  {/if}
  <div class="flex-1 min-h-0">
    <SplitPane direction="horizontal" ratio={0.5} min={0.2} max={0.8}>
      {#snippet primary()}
        <!-- The source is typed into: the pane is a well, and the editor is bare text on it. -->
        <div class="h-full w-full overflow-hidden bg-tln-well select-text">
          <CodeEditor
            variant="bare"
            label="Typst"
            {content}
            {language}
            {extensions}
            onChange={handleEditorChange}
            {onSave}
          />
        </div>
      {/snippet}
      {#snippet secondary()}
        <div class="h-full w-full overflow-hidden">
          <TypstPreview
            {pages}
            {diagnostics}
            {isCompiling}
            onExportPdf={handleExportPdf}
          />
        </div>
      {/snippet}
    </SplitPane>
  </div>
</div>
