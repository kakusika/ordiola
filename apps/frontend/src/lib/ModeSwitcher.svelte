<script lang="ts">
  // How the active document is edited. The modes that a document cannot be shown in are not offered.
  import { SegmentedControl } from "@telaion/primitives";
  import { FileText, Kanban, Palette, Zap } from "@telaion/primitives/icons";
  import type { EditorTab } from "$lib/types";

  type Mode = EditorTab["editMode"];

  let {
    mode,
    isTmt,
    onchange,
  }: {
    mode: Mode;
    /** Hybrid and full block editing exist for Tomet documents only. */
    isTmt: boolean;
    /** A person chose a mode. The page decides whether it can be entered. */
    onchange: (mode: Mode) => void;
  } = $props();

  const items = $derived([
    { value: "source", label: "ソース", icon: FileText, title: "ソース編集: 生テキスト直接編集" },
    ...(isTmt
      ? [
          { value: "hybrid", label: "ハイブリッド", icon: Zap, title: "ブロックハイブリッド編集" },
          { value: "full", label: "フルブロック", icon: Palette, title: "フルブロック (WYSIWYG) 編集" },
        ]
      : []),
    { value: "kanban", label: "カンバン", icon: Kanban, title: "カンバンボード表示" },
  ]);
</script>

<!-- The page may refuse a mode, so the selection is always the page's `mode`, never the click. -->
<SegmentedControl
  {items}
  size="sm"
  label="編集モード"
  bind:value={() => mode, (next) => onchange(next as Mode)}
/>
