<script lang="ts">
  import { Alert, Badge, Button, EmptyState, IconButton, Spinner, Toolbar } from "@telaion/primitives";
  import {
    ChevronDown,
    ChevronUp,
    FileDown,
    FileText,
    FileX,
    MoveHorizontal,
    ZoomIn,
    ZoomOut,
  } from "@telaion/primitives/icons";
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

  const ZOOM_MIN = 0.3;
  const ZOOM_MAX = 3.0;
  const ZOOM_STEP = 0.15;

  let zoom = $state<number>(1.0);
  let fitWidth = $state<boolean>(false);
  let isDiagnosticsOpen = $state<boolean>(true);

  const errorCount = $derived(diagnostics.filter((d) => d.severity === "error").length);
  const warningCount = $derived(diagnostics.filter((d) => d.severity === "warning").length);

  function zoomIn() {
    fitWidth = false;
    zoom = Math.min(ZOOM_MAX, +(zoom + ZOOM_STEP).toFixed(2));
  }

  function zoomOut() {
    fitWidth = false;
    zoom = Math.max(ZOOM_MIN, +(zoom - ZOOM_STEP).toFixed(2));
  }

  function resetZoom() {
    fitWidth = false;
    zoom = 1.0;
  }

  function toggleFitWidth() {
    fitWidth = !fitWidth;
  }
</script>

<div class="typst-preview h-full w-full flex flex-col bg-tln-canvas text-tln-fg overflow-hidden select-none">
  <!-- Preview controls -->
  <div class="p-2 shrink-0">
    <Toolbar label="Typst プレビュー">
      <div class="flex items-center gap-2 text-tln-xs text-tln-fg-muted">
        <span>Preview</span>
        {#if isCompiling}
          <Spinner label="コンパイル中..." />
          <span>コンパイル中...</span>
        {:else if pages.length > 0}
          <Badge>{pages.length} ページ</Badge>
        {/if}
      </div>

      <div class="flex items-center gap-1 mx-auto">
        <IconButton
          icon={ZoomOut}
          label="縮小"
          size="sm"
          onclick={zoomOut}
          disabled={zoom <= ZOOM_MIN}
        />
        <Button variant="quiet" size="sm" onclick={resetZoom} title="100%にリセット">
          {Math.round(zoom * 100)}%
        </Button>
        <IconButton
          icon={ZoomIn}
          label="拡大"
          size="sm"
          onclick={zoomIn}
          disabled={zoom >= ZOOM_MAX}
        />
        <Button
          variant={fitWidth ? "soft" : "quiet"}
          size="sm"
          icon={MoveHorizontal}
          aria-pressed={fitWidth}
          onclick={toggleFitWidth}
          title="幅に合わせる"
        >
          幅合致
        </Button>
      </div>

      {#if onExportPdf}
        <Button size="sm" icon={FileDown} onclick={onExportPdf} title="PDFをエクスポート">
          PDF 出力
        </Button>
      {/if}
    </Toolbar>
  </div>

  <!-- Pages -->
  <div class="flex-1 overflow-auto p-6 flex flex-col items-center gap-6 relative">
    {#if pages.length === 0 && !isCompiling}
      <div class="m-auto">
        {#if errorCount > 0}
          <EmptyState
            icon={FileX}
            title="プレビューを表示できません"
            description="ソースのエラーを直すと、自動的にコンパイルし直します"
          />
        {:else}
          <EmptyState
            icon={FileText}
            title="プレビューできるページがありません"
            description="Typst ソースコードを入力すると自動的にコンパイルされます"
          />
        {/if}
      </div>
    {:else}
      {#each pages as pageSvg, idx (idx)}
        <div class="flex flex-col items-center">
          <!-- The page is a printed sheet: paper in both themes. The SVG from the compiler draws its own ink. -->
          <div
            class="typst-page-container bg-tln-paper shadow-tln-content rounded-tln overflow-hidden transition-tln select-text"
            style="transform-origin: top center; {fitWidth ? 'width: 100%; max-width: calc(var(--tln-space) * 200);' : `transform: scale(${zoom});`}"
          >
            {@html pageSvg}
          </div>

          <span class="text-tln-xs text-tln-fg-muted mt-2 select-none">
            Page {idx + 1} / {pages.length}
          </span>
        </div>
      {/each}
    {/if}
  </div>

  <!-- Diagnostics (errors and warnings) -->
  {#if diagnostics.length > 0}
    <div class="typst-diagnostics bg-tln-content flex flex-col shrink-0 max-h-48 overflow-hidden">
      <div class="p-2 flex items-center justify-between gap-2 shrink-0">
        <div class="flex items-center gap-2">
          {#if errorCount > 0}
            <Badge tone="danger">{errorCount} エラー</Badge>
          {/if}
          {#if warningCount > 0}
            <Badge tone="warn">{warningCount} 警告</Badge>
          {/if}
        </div>
        <Button
          variant="quiet"
          size="sm"
          iconEnd={isDiagnosticsOpen ? ChevronDown : ChevronUp}
          aria-expanded={isDiagnosticsOpen}
          onclick={() => (isDiagnosticsOpen = !isDiagnosticsOpen)}
        >
          {isDiagnosticsOpen ? "隠す" : "詳細を表示"}
        </Button>
      </div>

      {#if isDiagnosticsOpen}
        <div class="flex-1 overflow-y-auto p-2 pt-0 flex flex-col gap-2 select-text">
          {#each diagnostics as diag, i (i)}
            <Alert tone={diag.severity === "error" ? "danger" : "warn"}>
              <div class="flex items-start gap-2">
                {#if onJumpToLine}
                  <Button
                    variant="quiet"
                    size="sm"
                    onclick={() => onJumpToLine(diag.line, diag.column)}
                    title="該当行へジャンプ"
                  >
                    [{diag.line}:{diag.column}]
                  </Button>
                {:else}
                  <span class="shrink-0">[{diag.line}:{diag.column}]</span>
                {/if}
                <span class="flex-1 min-w-0 break-words">{diag.message}</span>
              </div>
            </Alert>
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

  /* A divider between the preview and the diagnostics: one hairline, no frame. */
  .typst-diagnostics {
    border-top: var(--tln-hairline) solid var(--tln-border);
  }
</style>
