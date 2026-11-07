<script lang="ts">
  // The strip of open documents. Which documents are open, which one is active and what closing
  // one means (the unsaved-changes question) belong to the page; this only shows them.
  import { Tabs } from "@telaion/primitives";
  import { FileText, Ruler, ScrollText } from "@telaion/primitives/icons";
  import type { EditorTab } from "$lib/types";

  let {
    tabs,
    activeId,
    onselect,
    onclose,
  }: {
    tabs: EditorTab[];
    /** The id of the active tab; "" when none is. */
    activeId: string;
    onselect: (id: string) => void;
    /** A person asked to close this tab (its button, the Delete key or a middle click). */
    onclose: (tab: EditorTab) => void;
  } = $props();

  const items = $derived(
    tabs.map((tab) => ({
      id: tab.id,
      label: tab.title,
      icon: tab.isTmt ? ScrollText : tab.isTypst ? Ruler : FileText,
      title: tab.path,
      dirty: tab.isModified,
      closable: true,
    })),
  );

  function close(id: string) {
    const tab = tabs.find((t) => t.id === id);
    if (tab) onclose(tab);
  }
</script>

<Tabs {items} size="sm" label="開いているドキュメント" value={activeId} {onselect} onclose={close} />
