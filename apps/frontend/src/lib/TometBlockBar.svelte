<script lang="ts">
  // The insert bar of a Tomet document: one button per block, and the switch of the metadata inspector.
  import { Button, Toolbar } from "@telaion/primitives";
  import { ChevronDown, ChevronUp, Code, Info, Lightbulb, OctagonX, ScrollText, TriangleAlert } from "@telaion/primitives/icons";

  let {
    metaId,
    metaOpen = $bindable(false),
    controls,
    oninsert,
  }: {
    /** The id of the document, shown on the inspector switch; empty when it has none. */
    metaId: string;
    /** Whether the metadata inspector is open. */
    metaOpen?: boolean;
    /** The id of the element the switch opens and closes. */
    controls?: string;
    /** Called with the kind of block to insert: note, tip, warning, danger or code. */
    oninsert: (kind: "note" | "tip" | "warning" | "danger" | "code") => void;
  } = $props();
</script>

<div class="block-bar">
  <Toolbar label="Tomet ブロックの挿入">
    <span class="caption">Tomet ブロック</span>
    <!-- One meaning, one tone: a note informs, a tip is good news, a warning cautions, a danger destroys. -->
    <Button variant="soft" tone="accent" size="sm" icon={Info} title="Note (補足・注記) コールアウトを挿入" onclick={() => oninsert("note")}>Note</Button>
    <Button variant="soft" tone="ok" size="sm" icon={Lightbulb} title="Tip (ヒント) コールアウトを挿入" onclick={() => oninsert("tip")}>Tip</Button>
    <Button variant="soft" tone="warn" size="sm" icon={TriangleAlert} title="Warning (注意事項) コールアウトを挿入" onclick={() => oninsert("warning")}>Warning</Button>
    <Button variant="soft" tone="danger" size="sm" icon={OctagonX} title="Danger (重大・危険) コールアウトを挿入" onclick={() => oninsert("danger")}>Danger</Button>
    <Button variant="secondary" size="sm" icon={Code} title="コードブロック (code) を挿入" onclick={() => oninsert("code")}>Code Block</Button>
    <span class="end">
      <Button
        size="sm"
        icon={ScrollText}
        iconEnd={metaOpen ? ChevronUp : ChevronDown}
        title="文書メタデータ・エクスポート設定パネルを開閉"
        aria-expanded={metaOpen}
        aria-controls={controls}
        onclick={() => (metaOpen = !metaOpen)}
      >
        設定: {metaId || "未設定"}
      </Button>
    </span>
  </Toolbar>
</div>

<style>
  /* Room around the bar for the shadow of its surface. */
  .block-bar {
    flex: none;
    padding: calc(var(--tln-space) * 2) calc(var(--tln-space) * 3);
  }

  .caption {
    margin-right: var(--tln-space);
    font-size: var(--tln-text-xs);
    color: var(--tln-fg-muted);
    white-space: nowrap;
    user-select: none;
  }

  /* The switch of the inspector sits at the far end of the bar. */
  .end {
    margin-left: auto;
  }
</style>
