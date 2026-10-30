<script lang="ts">
  // The delete confirmation, built on @telaion/ui.
  import { Alert, Button, Icon, Modal } from "@telaion/ui";
  import { TriangleAlert } from "@telaion/ui/icons";

  let {
    entry,
    onConfirm,
    onCancel,
  }: {
    entry: { name: string; is_dir: boolean };
    onConfirm: () => void;
    onCancel: () => void;
  } = $props();
</script>

<Modal open title={entry.is_dir ? "フォルダの削除" : "ファイルの削除"} size="sm" onclose={onCancel}>
  <div class="message">
    <span class="mark"><Icon icon={TriangleAlert} /></span>
    <p>「<code>{entry.name}</code>」を削除してもよろしいですか？</p>
  </div>
  {#if entry.is_dir}
    <Alert tone="warn">フォルダ内のすべてのファイルおよびサブフォルダも完全に削除されます。</Alert>
  {/if}
  {#snippet footer()}
    <!-- Cancel takes the focus first: the safe answer is the default one. -->
    <Button autofocus onclick={onCancel}>キャンセル</Button>
    <Button variant="danger-fill" onclick={onConfirm}>削除</Button>
  {/snippet}
</Modal>

<style>
  .message {
    display: flex;
    align-items: flex-start;
    gap: calc(var(--tln-space) * 3);
    margin-bottom: calc(var(--tln-space) * 3);
  }

  .mark {
    display: inline-flex;
    padding: calc(var(--tln-space) * 2);
    font-size: var(--tln-text-md);
    color: var(--tln-danger);
    background: var(--tln-danger-soft);
    border: 1px solid var(--tln-danger);
    border-radius: var(--tln-radius);
  }

  p {
    margin: 0;
    min-width: 0;
    overflow-wrap: anywhere;
  }

  code {
    font-family: var(--tln-font-mono);
  }
</style>
