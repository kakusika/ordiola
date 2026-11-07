<script lang="ts">
  // Says, above the block editor, what the mode it is in does.
  import { Alert } from "@telaion/primitives";

  let { mode }: { mode: "hybrid" | "full" } = $props();
</script>

<Alert tone="info">
  {#if mode === "hybrid"}
    ブロックハイブリッドモード: 見出しに Tomet 構文記号（=[ ]=）が美しく添えられ、構造を意識したハイブリッド執筆が可能です。
  {:else}
    フルブロックモード (WYSIWYG): 構文記号を完全に隠し、モダンなリッチテキストとして見たまま執筆できます。
  {/if}
</Alert>
