<script lang="ts">
  // The bar over the main column: the open documents, the quick-open trigger, and, while a document
  // is open, how it is edited and the Save button. It holds controls only, so it is content: raised
  // over the canvas the document is on. State and decisions stay with the page.
  import { Button, Kbd } from "@telaion/primitives";
  import { Save, Search } from "@telaion/primitives/icons";
  import DocumentTabs from "$lib/DocumentTabs.svelte";
  import ModeSwitcher from "$lib/ModeSwitcher.svelte";
  import type { EditorTab } from "$lib/types";

  let {
    tabs,
    activeId,
    onselect,
    onclose,
    onquickopen,
    onmode,
    onsave,
  }: {
    tabs: EditorTab[];
    /** The id of the active tab; "" when no document is open. */
    activeId: string;
    onselect: (id: string) => void;
    onclose: (tab: EditorTab) => void;
    onquickopen: () => void;
    onmode: (mode: EditorTab["editMode"]) => void;
    onsave: () => void;
  } = $props();

  const active = $derived(tabs.find((tab) => tab.id === activeId) ?? null);
</script>

<header class="workspace-header">
  <div class="tabs">
    <DocumentTabs {tabs} {activeId} {onselect} {onclose} />
  </div>

  <Button size="sm" icon={Search} title="ファイルを検索 / クイックオープン (Ctrl+P)" onclick={onquickopen}>
    <span class="hidden sm:inline">ファイル検索</span>
    <Kbd keys={["Ctrl", "P"]} />
  </Button>

  {#if active}
    <ModeSwitcher mode={active.editMode} isTmt={active.isTmt} onchange={onmode} />
    <Button size="sm" icon={Save} title="ドキュメントを保存 (Ctrl+S)" onclick={onsave}>保存</Button>
  {/if}
</header>

<style>
  .workspace-header {
    position: relative;
    z-index: 1;
    flex: none;
    display: flex;
    align-items: center;
    gap: calc(var(--tln-space) * 2);
    padding: 0 calc(var(--tln-space) * 2);
    min-width: 0;
    color: var(--tln-fg);
    background: var(--tln-content);
    box-shadow: var(--tln-shadow-content);
    user-select: none;
  }

  /* The strip takes what the buttons leave and scrolls inside it. */
  .tabs {
    flex: 1;
    min-width: 0;
  }
</style>
