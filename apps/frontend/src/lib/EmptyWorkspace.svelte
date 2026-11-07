<script lang="ts">
  // What the main column shows while no document is open.
  import { Button, EmptyState } from "@telaion/primitives";
  import { FilePlus, ScrollText } from "@telaion/primitives/icons";

  let { oncreate }: { oncreate: () => void } = $props();
</script>

<div class="empty-workspace">
  <EmptyState
    icon={ScrollText}
    title="開いているドキュメントがありません"
    description="左側のエクスプローラーからファイルを選択して開くか、新しいファイルを作成してください。"
  >
    {#snippet action()}
      <Button icon={FilePlus} onclick={oncreate}>新しいファイルを作成</Button>
    {/snippet}
  </EmptyState>
</div>

<style>
  .empty-workspace {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 0;
  }
</style>
