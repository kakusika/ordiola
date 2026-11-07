<script lang="ts">
  /*
   * The vault in use, and the way to another: a button that names the vault and opens a menu of
   * the recent ones, with two ways to open one that is not listed (the system's folder picker, and
   * a typed path). The component only asks; opening a vault is the application's work.
   */
  import { Button, Field, IconButton, Input, Menu, Popover, Tooltip } from "@telaion/primitives";
  import type { MenuAction, MenuItem } from "@telaion/primitives";
  import { ChevronDown, FolderOpen, FolderPlus, FolderSearch, TextCursorInput } from "@telaion/primitives/icons";
  import type { VaultEntry } from "$lib/types";

  let {
    name,
    path,
    recent,
    onswitch,
    onpick,
  }: {
    /** The vault in use. */
    name: string;
    path: string;
    /** The vaults opened before, most recent first. */
    recent: VaultEntry[];
    /** A vault was chosen from the list, or its path was typed. */
    onswitch: (path: string) => void;
    /** The person asked for the system's folder picker. */
    onpick: () => void;
  } = $props();

  let menuOpen = $state(false);
  let pathOpen = $state(false);
  let typedPath = $state("");
  // Both popovers hang from the vault button; it is read back from the button itself.
  let vaultButton = $state<HTMLElement>();

  const items = $derived.by((): MenuItem[] => [
    ...(recent.length > 0
      ? [
          { type: "heading", label: "Recent Vaults" } as const,
          ...recent.map(
            (vault): MenuAction => ({
              // The row draws itself from the id, which is the path (see `vaultRow`).
              id: vault.path,
              label: vault.name,
              checked: vault.path === path,
              onselect: () => onswitch(vault.path),
            }),
          ),
          { type: "separator" } as const,
        ]
      : []),
    { label: "フォルダを選択して開く...", icon: FolderSearch, onselect: onpick },
    { label: "パスを入力して開く...", icon: TextCursorInput, onselect: () => (pathOpen = true) },
  ]);

  function openTyped(event: SubmitEvent) {
    event.preventDefault();
    const target = typedPath.trim();
    if (!target) return;
    pathOpen = false;
    typedPath = "";
    onswitch(target);
  }
</script>

{#snippet vaultRow(action: MenuAction)}
  {#if action.id}
    <span class="vault-row">
      <span class="vault-row-name">{action.label}</span>
      <span class="vault-row-path">{action.id}</span>
    </span>
  {:else}
    {action.label}
  {/if}
{/snippet}

<div class="vault-bar flex items-center gap-2 p-3">
  <div class="vault-trigger">
    <Popover bind:open={menuOpen} role="menu" label="Vault を切り替える" anchor={vaultButton} matchWidth>
      {#snippet trigger(popover)}
        <Tooltip text={path} placement="bottom">
          {#snippet children(describe)}
            <Button
              {...popover}
              {...describe}
              size="sm"
              icon={FolderOpen}
              iconEnd={ChevronDown}
              {@attach (element) => {
                vaultButton = element;
              }}
            >
              <span class="vault-name">{name}</span>
              <span class="vault-path">{path}</span>
            </Button>
          {/snippet}
        </Tooltip>
      {/snippet}
      <div class="vault-menu">
        <Menu {items} item={vaultRow} />
      </div>
    </Popover>
  </div>

  <IconButton icon={FolderPlus} label="ネイティブフォルダ選択でVaultを開く" size="sm" variant="secondary" onclick={onpick} />

  <Popover bind:open={pathOpen} label="Vault のパスを入力" anchor={vaultButton} matchWidth>
    <form class="flex items-end gap-2" onsubmit={openTyped}>
      <div class="min-w-0 flex-1">
        <Field label="直接パスを入力">
          <Input bind:value={typedPath} mono size="sm" placeholder="/path/to/vault" spellcheck={false} autocomplete="off" />
        </Field>
      </div>
      <Button type="submit" variant="primary" size="sm" disabled={!typedPath.trim()}>開く</Button>
    </form>
  </Popover>
</div>

<style>
  /* The vault bar is a divider's width above the explorer. */
  .vault-bar {
    border-bottom: var(--tln-hairline) solid var(--tln-border);
  }

  /* The vault button takes the width the folder button leaves, and its path gives way first. */
  .vault-trigger {
    display: flex;
    flex: 1;
    min-width: 0;
  }
  .vault-trigger :global(button[aria-haspopup]) {
    flex: 1;
    min-width: 0;
  }

  .vault-name {
    flex: none;
    font-weight: var(--tln-weight-strong);
  }
  .vault-path {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    text-align: left;
    font-family: var(--tln-font-mono);
    color: var(--tln-fg-muted);
  }

  /* A long path shortens inside the menu instead of widening it. */
  .vault-menu {
    max-width: calc(var(--tln-space) * 90);
  }
  .vault-row {
    display: flex;
    align-items: baseline;
    gap: calc(var(--tln-space) * 2);
    min-width: 0;
  }
  .vault-row-name {
    flex: none;
  }
  .vault-row-path {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    font-family: var(--tln-font-mono);
    font-size: var(--tln-text-xs);
    color: var(--tln-fg-muted);
  }
</style>
