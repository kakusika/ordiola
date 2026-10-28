<script lang="ts">
  import FileTreeNode from "./FileTreeNode.svelte";
  import type { FileEntry, InlineAction } from "./types";

  interface Props {
    entry: FileEntry;
    activePath: string;
    selectedPath: string | null;
    depth?: number;
    inlineAction: InlineAction | null;
    onSelectFile: (file: FileEntry) => void;
    onSelectEntry: (entry: FileEntry) => void;
    onToggleDir: (entry: FileEntry) => Promise<void>;
    onContextMenu: (e: MouseEvent, entry: FileEntry) => void;
    onCommitInlineAction: (value: string) => Promise<void>;
    onCancelInlineAction: () => void;
    onMoveEntry: (sourceRel: string, destDirRel: string) => Promise<void>;
  }

  let {
    entry,
    activePath,
    selectedPath,
    depth = 0,
    inlineAction,
    onSelectFile,
    onSelectEntry,
    onToggleDir,
    onContextMenu,
    onCommitInlineAction,
    onCancelInlineAction,
    onMoveEntry,
  }: Props = $props();

  let isDragOver = $state(false);

  let isRenaming = $derived(
    inlineAction?.type === "rename" &&
      (inlineAction.targetEntry?.path === entry.path ||
        inlineAction.targetEntry?.relative_path === entry.relative_path)
  );

  let isCreatingChildHere = $derived(
    entry.is_dir &&
      entry.isExpanded &&
      inlineAction !== null &&
      (inlineAction.type === "new-file" || inlineAction.type === "new-folder") &&
      (inlineAction.parentPath === entry.relative_path ||
        inlineAction.parentPath === entry.path)
  );

  let renameValue = $state("");
  let newChildValue = $state("");
  let isCommitting = false;

  $effect(() => {
    if (isRenaming) {
      renameValue = entry.name;
    }
  });

  $effect(() => {
    if (isCreatingChildHere) {
      newChildValue = "";
    }
  });

  function focusSelect(node: HTMLInputElement) {
    node.focus();
    if (!entry.is_dir) {
      const dotIndex = entry.name.lastIndexOf(".");
      if (dotIndex > 0) {
        node.setSelectionRange(0, dotIndex);
        return;
      }
    }
    node.select();
  }

  function focusInput(node: HTMLInputElement) {
    node.focus();
  }

  function handleClick(e: MouseEvent) {
    e.stopPropagation();
    onSelectEntry(entry);
    if (entry.is_dir) {
      onToggleDir(entry);
    } else {
      onSelectFile(entry);
    }
  }

  function handleContextMenu(e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    onSelectEntry(entry);
    onContextMenu(e, entry);
  }

  async function handleRenameKeyDown(e: KeyboardEvent) {
    if (e.key === "Enter") {
      e.preventDefault();
      e.stopPropagation();
      const val = renameValue.trim();
      if (val && val !== entry.name) {
        isCommitting = true;
        await onCommitInlineAction(val);
        isCommitting = false;
      } else {
        onCancelInlineAction();
      }
    } else if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      onCancelInlineAction();
    }
  }

  async function handleNewChildKeyDown(e: KeyboardEvent) {
    if (e.key === "Enter") {
      e.preventDefault();
      e.stopPropagation();
      const val = newChildValue.trim();
      if (val) {
        isCommitting = true;
        await onCommitInlineAction(val);
        isCommitting = false;
      } else {
        onCancelInlineAction();
      }
    } else if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      onCancelInlineAction();
    }
  }

  function handleBlur() {
    setTimeout(() => {
      if (!isCommitting) {
        onCancelInlineAction();
      }
    }, 150);
  }

  function handleDragStart(e: DragEvent) {
    if (isRenaming) return;
    const path = entry.relative_path || entry.path;
    e.dataTransfer?.setData("text/plain", path);
    if (e.dataTransfer) {
      e.dataTransfer.effectAllowed = "move";
    }
  }

  function handleDragOver(e: DragEvent) {
    if (!entry.is_dir) return;
    e.preventDefault();
    if (e.dataTransfer) {
      e.dataTransfer.dropEffect = "move";
    }
  }

  function handleDragEnter(e: DragEvent) {
    if (!entry.is_dir) return;
    e.preventDefault();
    isDragOver = true;
  }

  function handleDragLeave(e: DragEvent) {
    if (!entry.is_dir) return;
    isDragOver = false;
  }

  async function handleDrop(e: DragEvent) {
    if (!entry.is_dir) return;
    e.preventDefault();
    e.stopPropagation();
    isDragOver = false;
    const sourcePath = e.dataTransfer?.getData("text/plain");
    const targetDir = entry.relative_path || entry.path;
    if (sourcePath && sourcePath !== targetDir) {
      await onMoveEntry(sourcePath, targetDir);
    }
  }
</script>

<div class="select-none text-xs">
  {#if isRenaming}
    <div
      class="w-full flex items-center gap-1.5 py-1 px-1.5 rounded bg-stone-900 border border-sky-500 shadow-sm"
      style="padding-left: {depth * 14 + 6}px;"
    >
      <span class="w-3 text-center text-[10px] text-stone-400 shrink-0">
        {#if entry.is_dir}
          {entry.isExpanded ? "▼" : "▶"}
        {/if}
      </span>
      <span class="text-sm shrink-0">
        {#if entry.is_dir}
          {entry.isExpanded ? "📂" : "📁"}
        {:else if entry.name.endsWith(".tmt")}
          📜
        {:else}
          📄
        {/if}
      </span>
      <input
        type="text"
        bind:value={renameValue}
        class="flex-1 bg-stone-950 border border-sky-500 rounded px-1.5 py-0.5 text-[11px] font-mono text-stone-100 focus:outline-none"
        onkeydown={handleRenameKeyDown}
        onblur={handleBlur}
        use:focusSelect
      />
    </div>
  {:else}
    <button
      class="w-full flex items-center gap-1.5 py-1 px-1.5 rounded transition text-left group cursor-pointer
        {isDragOver
          ? 'bg-sky-500/30 ring-2 ring-sky-400 text-sky-100 shadow-md'
          : activePath === entry.path || activePath === entry.relative_path
            ? 'bg-stone-800 text-sky-300 font-medium shadow-sm'
            : selectedPath === entry.path || selectedPath === entry.relative_path
              ? 'bg-stone-800/70 text-stone-100 ring-1 ring-stone-700/80'
              : 'text-stone-300 hover:bg-stone-800/40 hover:text-stone-100'}"
      style="padding-left: {depth * 14 + 6}px;"
      draggable={!isRenaming}
      ondragstart={handleDragStart}
      ondragover={handleDragOver}
      ondragenter={handleDragEnter}
      ondragleave={handleDragLeave}
      ondrop={handleDrop}
      onclick={handleClick}
      oncontextmenu={handleContextMenu}
    >
      {#if entry.is_dir}
        <span class="w-3 text-center text-[10px] text-stone-400 shrink-0">
          {#if entry.isLoading}
            <span class="animate-spin inline-block">⟳</span>
          {:else if entry.isExpanded}
            ▼
          {:else}
            ▶
          {/if}
        </span>
        <span class="text-sm shrink-0">{entry.isExpanded ? "📂" : "📁"}</span>
        <span class="truncate font-mono text-[11px] font-semibold text-stone-300 group-hover:text-stone-100">
          {entry.name}
        </span>
      {:else}
        <span class="w-3 shrink-0"></span>
        <span class="text-sm shrink-0">
          {#if entry.name.endsWith(".tmt")}
            📜
          {:else}
            📄
          {/if}
        </span>
        <span class="truncate font-mono text-[11px] {entry.name.endsWith('.tmt') ? 'text-stone-200' : 'text-stone-400'}">
          {entry.name}
        </span>
      {/if}
    </button>
  {/if}

  {#if entry.is_dir && entry.isExpanded}
    <div class="flex flex-col">
      <!-- Inline input for creating a new child inside this directory -->
      {#if isCreatingChildHere}
        <div
          class="flex items-center gap-1.5 py-1 px-1.5 rounded bg-stone-900 border border-sky-500 shadow-sm my-0.5"
          style="padding-left: {(depth + 1) * 14 + 6}px;"
        >
          <span class="w-3 shrink-0"></span>
          <span class="text-sm shrink-0">
            {inlineAction?.type === "new-folder" ? "📁" : "📄"}
          </span>
          <input
            type="text"
            bind:value={newChildValue}
            placeholder={inlineAction?.type === "new-folder" ? "フォルダ名" : "ファイル名.tmt"}
            class="flex-1 bg-stone-950 border border-sky-500 rounded px-1.5 py-0.5 text-[11px] font-mono text-stone-100 focus:outline-none"
            onkeydown={handleNewChildKeyDown}
            onblur={handleBlur}
            use:focusInput
          />
        </div>
      {/if}

      {#if entry.children}
        {#each entry.children as child (child.path || child.relative_path)}
          <FileTreeNode
            entry={child}
            {activePath}
            {selectedPath}
            depth={depth + 1}
            {inlineAction}
            {onSelectFile}
            {onSelectEntry}
            {onToggleDir}
            {onContextMenu}
            {onCommitInlineAction}
            {onCancelInlineAction}
            {onMoveEntry}
          />
        {/each}
        {#if entry.children.length === 0 && !isCreatingChildHere}
          <div
            class="text-[10px] text-stone-500 italic py-1 font-mono"
            style="padding-left: {(depth + 1) * 14 + 20}px;"
          >
            (空のディレクトリ)
          </div>
        {/if}
      {/if}
    </div>
  {/if}
</div>
