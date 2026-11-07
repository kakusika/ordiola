<script lang="ts">
  // The foot of the sidebar: which backend the app is running on, and the last thing it reported.
  let {
    backend,
    message = "",
  }: {
    backend: string;
    message?: string;
  } = $props();
</script>

<div class="status-bar text-tln-xs text-tln-fg-muted">
  <div class="flex items-center justify-between gap-2">
    <span>Backend:</span>
    <span class="font-tln-mono text-tln-ok">{backend}</span>
  </div>
  {#if message}
    <div class="message truncate font-tln-mono" title={message}>{message}</div>
  {/if}
</div>

<style>
  /* It holds text only, so it is no surface of its own: it sits on the sidebar, set off by a divider. */
  .status-bar {
    display: flex;
    flex-direction: column;
    gap: calc(var(--tln-space) * 1.5);
    padding: calc(var(--tln-space) * 3);
    border-top: var(--tln-hairline) solid var(--tln-border);
  }
</style>
