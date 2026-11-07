<script lang="ts">
  /*
   * The file explorer: a header of four actions, and the tree of the vault. The pane owns what is
   * on the screen (which row is being named, which menu is open, what is selected and expanded) and
   * asks the application for everything that touches files, through the callbacks. A name typed in
   * the tree is answered by the application: a message in return refuses it and stays under the field.
   */
  import { ContextMenu, IconButton, Spinner } from "@telaion/primitives";
  import type { MenuItem } from "@telaion/primitives";
  import {
    ChevronsDownUp,
    File,
    FilePlus,
    Folder,
    FolderPlus,
    Pencil,
    RefreshCw,
    Ruler,
    ScrollText,
    Trash,
  } from "@telaion/primitives/icons";
  import { TreeExplorer } from "@telaion/views";
  import type { TreeCheckResult, TreeCommitEvent, TreeContextMenuEvent, TreeEditing, TreeMoveEvent, TreeNode } from "@telaion/views";
  import { childrenOf, creationDir, entryPath, findEntry, joinPath, parentPath, pathChain } from "$lib/explorer";
  import type { NewEntryKind } from "$lib/explorer";
  import type { FileEntry } from "$lib/types";

  let {
    entries,
    selectedId = $bindable(null),
    expandedIds = $bindable(new Set<string>()),
    refreshing = false,
    onload,
    onopen,
    oncreate,
    onrename,
    onmove,
    ondelete,
    onrefresh,
  }: {
    /** The root of the vault. A directory whose `children` are not set has not been listed yet. */
    entries: FileEntry[];
    /** The path of the selected row. */
    selectedId?: string | null;
    /** The paths of the open directories. */
    expandedIds?: Set<string>;
    /** The tree is being read again. */
    refreshing?: boolean;
    /** A directory was opened: list its children (and list them again if they are loaded). */
    onload: (directory: FileEntry) => Promise<void>;
    /** A file was chosen. */
    onopen: (file: FileEntry) => void;
    /** A name was typed for a new file or folder in `directory` (`""` is the root). A message refuses it. */
    oncreate: (directory: string, name: string, kind: NewEntryKind) => Promise<string | void>;
    /** A new name was typed for `entry`. A message refuses it. */
    onrename: (entry: FileEntry, name: string) => Promise<string | void>;
    /** `entry` was dropped into `directory` (`""` is the root). */
    onmove: (entry: FileEntry, directory: string) => void;
    /** The person asked to delete `entry`. */
    ondelete: (entry: FileEntry) => void;
    /** The person asked to read the tree again. */
    onrefresh: () => void;
  } = $props();

  let tree = $state<ReturnType<typeof TreeExplorer>>();
  let editing = $state<TreeEditing | null>(null);
  // What a create row creates. The tree only knows that it names a new child.
  let creating = $state<NewEntryKind>("file");
  let menu = $state<{ open: boolean; x: number; y: number; node: TreeNode | null }>({
    open: false,
    x: 0,
    y: 0,
    node: null,
  });

  // ---- The tree as nodes ---------------------------------------------------------------------

  function fileIcon(name: string) {
    return name.endsWith(".tmt") ? ScrollText : name.endsWith(".typ") ? Ruler : File;
  }

  function toNodes(list: FileEntry[]): TreeNode[] {
    return list.map((entry) => ({
      id: entryPath(entry),
      label: entry.name,
      isFolder: entry.is_dir,
      // No children yet is not an empty directory.
      children: entry.children ? toNodes(entry.children) : undefined,
      icon: entry.is_dir ? undefined : fileIcon(entry.name),
      loading: entry.isLoading,
      data: entry,
    }));
  }
  const nodes = $derived(toNodes(entries));

  const entryOf = (node: TreeNode) => node.data as FileEntry;

  // ---- Selecting, opening ---------------------------------------------------------------------

  function handleSelect(node: TreeNode) {
    const entry = entryOf(node);
    if (!entry.is_dir) onopen(entry);
  }

  function handleToggle(node: TreeNode, expanded: boolean) {
    if (expanded) void onload(entryOf(node));
  }

  // A click on the empty area of the pane selects nothing: the next new file then goes to the root.
  function handlePaneClick(event: MouseEvent) {
    const target = event.target as Element;
    if (!target.closest('[role="treeitem"], [data-tln-edit]')) selectedId = null;
  }

  // The tree has no key for delete. Delete on a row asks to delete that row.
  function handlePaneKeydown(event: KeyboardEvent) {
    if (event.key !== "Delete" || event.defaultPrevented) return;
    const target = event.target as Element;
    if (target.closest("[data-tln-edit]")) return;
    const row = target.closest<HTMLElement>('[role="treeitem"]');
    const entry = findEntry(entries, row?.dataset.id ?? null);
    if (!entry) return;
    event.preventDefault();
    ondelete(entry);
  }

  // ---- Opening directories --------------------------------------------------------------------

  /** Opens `directory` and everything above it, listing what is not listed yet. */
  async function reveal(directory: string) {
    const chain = pathChain(directory);
    if (chain.length === 0) return;
    expandedIds = new Set([...expandedIds, ...chain]);
    for (const path of chain) {
      const entry = findEntry(entries, path);
      if (entry?.is_dir && !entry.children) await onload(entry);
    }
  }

  export function collapseAll() {
    expandedIds = new Set();
  }

  // ---- Naming: creating and renaming ----------------------------------------------------------

  /**
   * Starts naming a new file or folder. It goes in `directory` (`""` is the root), by default the
   * selected directory, or the directory of the selected file.
   */
  export async function startCreate(kind: NewEntryKind, directory: string = creationDir(entries, selectedId)) {
    await reveal(directory);
    creating = kind;
    editing = {
      kind: "create",
      parentId: directory || null,
      icon: kind === "folder" ? Folder : File,
      label: kind === "folder" ? "新しいフォルダの名前" : "新しいファイルの名前",
      placeholder: kind === "folder" ? "フォルダ名" : "ファイル名.tmt",
    };
  }

  /** Starts renaming `entry`. A file's extension is left out of the selection. */
  export function startRename(entry: FileEntry) {
    const dot = entry.name.lastIndexOf(".");
    editing = {
      kind: "rename",
      id: entryPath(entry),
      label: "新しい名前",
      selection: !entry.is_dir && dot > 0 ? [0, dot] : undefined,
    };
  }

  function validate(value: string, current: TreeEditing): TreeCheckResult {
    if (/[\\/]/.test(value)) return "名前に / と \\ は使えません";
    if (value === "." || value === "..") return "この名前は使えません";
    const directory = current.kind === "create" ? (current.parentId ?? "") : parentPath(current.id);
    const siblings = childrenOf(entries, directory);
    if (siblings?.some((sibling) => sibling.name === value)) return "同じ名前がすでにあります";
  }

  async function handleCommit({ editing: current, value }: TreeCommitEvent): Promise<TreeCheckResult> {
    if (current.kind === "create") return oncreate(current.parentId ?? "", value, creating);
    const entry = findEntry(entries, current.id);
    if (!entry) return;
    const refused = await onrename(entry, value);
    // A renamed row is a new row (its id is its path): the focus the old one had goes to it.
    if (!refused) void tree?.focusNode(joinPath(parentPath(current.id), value));
    return refused;
  }

  function handleMove({ source, targetId }: TreeMoveEvent) {
    onmove(entryOf(source), targetId ?? "");
  }

  // ---- The context menu ------------------------------------------------------------------------

  function handleContextMenu({ node, x, y }: TreeContextMenuEvent) {
    menu = { open: true, x, y, node };
  }

  const items = $derived.by((): MenuItem[] => {
    const node = menu.node;
    if (!node) {
      return [
        { label: "新しいファイル...", icon: FilePlus, onselect: () => startCreate("file", "") },
        { label: "新しいフォルダ...", icon: FolderPlus, onselect: () => startCreate("folder", "") },
        { type: "separator" },
        { label: "すべて折りたたむ", icon: ChevronsDownUp, onselect: collapseAll },
        { label: "エクスプローラー更新", icon: RefreshCw, onselect: onrefresh },
      ];
    }
    const entry = entryOf(node);
    return [
      { type: "heading", label: entry.name },
      ...(entry.is_dir
        ? ([
            { label: "新しいファイル...", icon: FilePlus, onselect: () => startCreate("file", entryPath(entry)) },
            { label: "新しいフォルダ...", icon: FolderPlus, onselect: () => startCreate("folder", entryPath(entry)) },
            { type: "separator" },
          ] satisfies MenuItem[])
        : []),
      { label: "名前の変更", icon: Pencil, shortcut: "F2", onselect: () => startRename(entry) },
      { type: "separator" },
      { label: "削除...", icon: Trash, shortcut: "Delete", danger: true, onselect: () => ondelete(entry) },
    ];
  });
</script>

<section class="explorer flex min-h-0 flex-1 flex-col" aria-label="エクスプローラー">
  <div class="explorer-header flex items-center justify-between px-3 py-2">
    <h2 class="text-tln-xs font-tln-strong tracking-wider text-tln-fg-muted uppercase">Explorer</h2>
    <div class="flex items-center gap-1">
      <IconButton icon={FilePlus} label="新しいファイルを作成" size="sm" onclick={() => startCreate("file")} />
      <IconButton icon={FolderPlus} label="新しいフォルダを作成" size="sm" onclick={() => startCreate("folder")} />
      {#if refreshing}
        <span class="refreshing"><Spinner label="エクスプローラーを再読み込み中" /></span>
      {:else}
        <IconButton icon={RefreshCw} label="エクスプローラーを再読み込み" size="sm" onclick={onrefresh} />
      {/if}
      <IconButton icon={ChevronsDownUp} label="すべてのフォルダを折りたたむ" size="sm" onclick={collapseAll} />
    </div>
  </div>

  <!-- The pane only listens to what happens inside the tree; the tree has the roles. -->
  <div class="min-h-0 flex-1 overflow-y-auto p-2" role="presentation" onclick={handlePaneClick} onkeydown={handlePaneKeydown}>
    <TreeExplorer
      bind:this={tree}
      {nodes}
      label="ファイル"
      bind:selectedId
      bind:expandedIds
      bind:editing
      contextId={menu.open ? (menu.node?.id ?? null) : null}
      {validate}
      oncommit={handleCommit}
      onselect={handleSelect}
      ontoggle={handleToggle}
      onrename={(node) => startRename(entryOf(node))}
      onmove={handleMove}
      oncontextmenu={handleContextMenu}
    >
      {#snippet empty(parent)}
        {parent ? "(空のディレクトリ)" : "ファイルが見つかりません"}
      {/snippet}
    </TreeExplorer>
  </div>

  <ContextMenu bind:open={menu.open} bind:x={menu.x} bind:y={menu.y} {items} label="ファイル操作" />
</section>

<style>
  .explorer-header {
    border-bottom: var(--tln-hairline) solid var(--tln-border);
  }

  /*
   * The refresh button while it reads: a spinner the size of the button, so the row of buttons
   * does not move. (IconButton size="sm": a step of padding around an icon of the small type.)
   */
  .refreshing {
    display: inline-flex;
    padding: calc(var(--tln-space) * 1.5);
    font-size: var(--tln-text-sm);
    line-height: 1;
  }
</style>
