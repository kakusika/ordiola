<script lang="ts">
  import type { TypstDiagnostic } from "./types";

  interface Props {
    pages?: string[];
    diagnostics?: TypstDiagnostic[];
    isCompiling?: boolean;
    onJumpToLine?: (line: number, column: number) => void;
    onExportPdf?: () => void;
  }

  let {
    pages = [],
    diagnostics = [],
    isCompiling = false,
    onJumpToLine,
    onExportPdf,
  }: Props = $props();

  let zoom = $state<number>(1.0);
  let fitWidth = $state<boolean>(false);
  let isDiagnosticsOpen = $state<boolean>(true);

  const errorCount = $derived(diagnostics.filter((d) => d.severity === "error").length);
  const warningCount = $derived(diagnostics.filter((d) => d.severity === "warning").length);

  function zoomIn() {
    fitWidth = false;
    zoom = Math.min(3.0, +(zoom + 0.15).toFixed(2));
  }

  function zoomOut() {
    fitWidth = false;
    zoom = Math.max(0.3, +(zoom - 0.15).toFixed(2));
  }

  function resetZoom() {
    fitWidth = false;
    zoom = 1.0;
  }

  function toggleFitWidth() {
    fitWidth = !fitWidth;
  }
</script>

<div class="h-full w-full flex flex-col bg-stone-900 overflow-hidden select-none">
  <!-- Preview Control Header -->
  <header class="h-9 px-3 border-b border-stone-800 bg-stone-950/80 flex items-center justify-between text-xs text-stone-300 shrink-0 backdrop-blur-xs gap-2">
    <!-- Left: Status & Page count -->
    <div class="flex items-center gap-2">
      <span class="font-semibold text-stone-400 text-[11px] uppercase tracking-wider">Preview</span>
      {#if isCompiling}
        <span class="flex items-center gap-1 text-[11px] text-sky-400 animate-pulse">
          <svg class="w-3 h-3 animate-spin" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M21 12a9 9 0 1 1-6.219-8.56" />
          </svg>
          コンパイル中...
        </span>
      {:else if pages.length > 0}
        <span class="text-[11px] text-stone-400 font-mono">
          {pages.length} ページ
        </span>
      {/if}
    </div>

    <!-- Center: Zoom Controls -->
    <div class="flex items-center gap-1 bg-stone-900 border border-stone-800 rounded px-1 py-0.5">
      <button
        class="w-5 h-5 flex items-center justify-center rounded hover:bg-stone-800 text-stone-400 hover:text-stone-200 cursor-pointer disabled:opacity-40"
        onclick={zoomOut}
        disabled={zoom <= 0.3}
        title="縮小"
      >
        −
      </button>

      <button
        class="px-1.5 text-[11px] font-mono text-stone-300 hover:text-white cursor-pointer"
        onclick={resetZoom}
        title="100%にリセット"
      >
        {Math.round(zoom * 100)}%
      </button>

      <button
        class="w-5 h-5 flex items-center justify-center rounded hover:bg-stone-800 text-stone-400 hover:text-stone-200 cursor-pointer disabled:opacity-40"
        onclick={zoomIn}
        disabled={zoom >= 3.0}
        title="拡大"
      >
        ＋
      </button>

      <div class="w-px h-3 bg-stone-800 mx-0.5"></div>

      <button
        class="px-1.5 py-0.5 rounded text-[10px] cursor-pointer transition font-mono {fitWidth ? 'bg-sky-500/20 text-sky-300 font-medium' : 'text-stone-400 hover:text-stone-200'}"
        onclick={toggleFitWidth}
        title="幅に合わせる"
      >
        幅合致
      </button>
    </div>

    <!-- Right: Actions (Export PDF) -->
    <div class="flex items-center gap-2">
      {#if onExportPdf}
        <button
          class="flex items-center gap-1 px-2 py-0.5 rounded bg-sky-950/60 hover:bg-sky-900/80 text-sky-300 hover:text-sky-100 border border-sky-500/30 text-[11px] font-medium transition cursor-pointer shadow-sm"
          onclick={onExportPdf}
          title="PDFをエクスポート"
        >
          <span>📑</span>
          <span>PDF 出力</span>
        </button>
      {/if}
    </div>
  </header>

  <!-- Pages Canvas View Area -->
  <div class="flex-1 overflow-auto p-6 flex flex-col items-center gap-6 bg-stone-900/90 relative">
    {#if pages.length === 0 && !isCompiling}
      <div class="m-auto text-center text-stone-500 text-xs py-12">
        <div class="text-3xl mb-2">📄</div>
        <p>プレビューできるページがありません</p>
        <p class="text-[11px] text-stone-600 mt-1">Typst ソースコードを入力すると自動的にコンパイルされます</p>
      </div>
    {:else}
      {#each pages as pageSvg, idx (idx)}
        <div class="flex flex-col items-center">
          <!-- Page Paper Container -->
          <div
            class="typst-page-container bg-white shadow-2xl rounded-xs overflow-hidden transition-all duration-75 ease-out select-text"
            style="transform-origin: top center; {fitWidth ? 'width: 100%; max-width: 820px;' : `transform: scale(${zoom});`}"
          >
            {@html pageSvg}
          </div>

          <!-- Page Indicator Footer -->
          <span class="text-[10px] font-mono text-stone-500 mt-2 select-none">
            Page {idx + 1} / {pages.length}
          </span>
        </div>
      {/each}
    {/if}
  </div>

  <!-- Diagnostics Bottom Banner (Errors & Warnings) -->
  {#if diagnostics.length > 0}
    <div class="border-t border-stone-800 bg-stone-950/95 flex flex-col text-xs shrink-0 max-h-48 overflow-hidden">
      <!-- Diagnostics Header Toggle -->
      <button
        class="h-7 px-3 flex items-center justify-between text-stone-400 hover:text-stone-200 cursor-pointer bg-stone-900/50 border-b border-stone-800/60"
        onclick={() => (isDiagnosticsOpen = !isDiagnosticsOpen)}
      >
        <div class="flex items-center gap-2">
          {#if errorCount > 0}
            <span class="flex items-center gap-1 text-rose-400 font-semibold text-[11px]">
              <span>🛑</span>
              <span>{errorCount} エラー</span>
            </span>
          {/if}
          {#if warningCount > 0}
            <span class="flex items-center gap-1 text-amber-400 font-medium text-[11px]">
              <span>⚠️</span>
              <span>{warningCount} 警告</span>
            </span>
          {/if}
        </div>
        <span class="text-[10px] text-stone-500 font-mono">
          {isDiagnosticsOpen ? "▼ 隠す" : "▲ 詳細を表示"}
        </span>
      </button>

      {#if isDiagnosticsOpen}
        <div class="flex-1 overflow-y-auto p-2 space-y-1 font-mono text-[11px]">
          {#each diagnostics as diag, i (i)}
            <button
              class="w-full text-left p-1.5 rounded flex items-start gap-2 hover:bg-stone-900 transition cursor-pointer border border-transparent hover:border-stone-800 {diag.severity === 'error' ? 'text-rose-300' : 'text-amber-300'}"
              onclick={() => onJumpToLine?.(diag.line, diag.column)}
              title="該当行へジャンプ"
            >
              <span class="shrink-0 text-xs">{diag.severity === "error" ? "✖" : "⚠"}</span>
              <span class="text-stone-400 shrink-0 font-mono">[{diag.line}:{diag.column}]</span>
              <span class="flex-1 break-words leading-relaxed text-stone-200">{diag.message}</span>
            </button>
          {/each}
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  :global(.typst-page-container svg) {
    display: block;
    width: 100%;
    height: auto;
  }
</style>
