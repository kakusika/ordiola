<script lang="ts">
  // Deleting a file or a folder. What is ordiola's own here is the wording and the extra warning for a
  // folder; the dialog, its tone and the safe focus are ConfirmDialog's.
  import { Alert, ConfirmDialog } from "@telaion/primitives";

  let {
    entry,
    onconfirm,
    oncancel,
  }: {
    entry: { name: string; is_dir: boolean };
    onconfirm: () => void;
    oncancel: () => void;
  } = $props();
</script>

{#snippet message()}
  <p>「<code>{entry.name}</code>」を削除してもよろしいですか？</p>
  {#if entry.is_dir}
    <Alert tone="warn">フォルダ内のすべてのファイルおよびサブフォルダも完全に削除されます。</Alert>
  {/if}
{/snippet}

<ConfirmDialog
  open
  destructive
  title={entry.is_dir ? "フォルダの削除" : "ファイルの削除"}
  {message}
  confirm="削除"
  cancel="キャンセル"
  {onconfirm}
  {oncancel}
/>

<style>
  p {
    margin: 0;
  }

  p + :global(*) {
    margin-top: calc(var(--tln-space) * 3);
  }

  code {
    font-family: var(--tln-font-mono);
  }
</style>
