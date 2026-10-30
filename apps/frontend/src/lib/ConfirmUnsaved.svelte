<script lang="ts">
  // The "unsaved changes on tab close" dialog, built on @telaion/ui.
  import { Button, Icon, Modal } from "@telaion/ui";
  import { TriangleAlert } from "@telaion/ui/icons";

  let {
    title,
    onSave,
    onDiscard,
    onCancel,
  }: {
    title: string;
    onSave: () => void;
    onDiscard: () => void;
    onCancel: () => void;
  } = $props();
</script>

<Modal open title="変更が保存されていません" size="sm" onclose={onCancel}>
  <div class="message">
    <span class="mark"><Icon icon={TriangleAlert} /></span>
    <p>「<code>{title}</code>」への変更を保存しますか？</p>
  </div>
  {#snippet footer()}
    <Button onclick={onCancel}>キャンセル</Button>
    <Button variant="danger" onclick={onDiscard}>保存しない</Button>
    <Button variant="primary" autofocus onclick={onSave}>保存する</Button>
  {/snippet}
</Modal>

<style>
  .message {
    display: flex;
    align-items: flex-start;
    gap: calc(var(--tln-space) * 3);
  }

  .mark {
    display: inline-flex;
    padding: calc(var(--tln-space) * 2);
    font-size: var(--tln-text-md);
    color: var(--tln-warn);
    background: var(--tln-warn-soft);
    border: 1px solid var(--tln-warn);
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
