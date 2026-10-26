<script lang="ts">
  import FileTreeNode from "./FileTreeNode.svelte";

  export interface FileEntry {
    name: string;
    path: string;
    relative_path: string;
    is_dir: boolean;
    extension?: string | null;
    has_children: boolean;
    children?: FileEntry[] | null;
    isExpanded?: boolean;
    isLoading?: boolean;
  }

  let {
    entry,
    activePath,
    depth = 0,
    onSelectFile,
    onToggleDir,
  } = $props<{
    entry: FileEntry;
    activePath: string;
    depth?: number;
    onSelectFile: (file: FileEntry) => void;
    onToggleDir: (entry: FileEntry) => Promise<void>;
  }>();

  function handleClick() {
    if (entry.is_dir) {
      onToggleDir(entry);
    } else {
      onSelectFile(entry);
    }
  }
</script>

<div class="select-none text-xs">
  <button
    class="w-full flex items-center gap-1.5 py-1 px-1.5 rounded transition text-left group
      {activePath === entry.path || activePath === entry.relative_path
        ? 'bg-stone-800 text-sky-300 font-medium shadow-sm'
        : 'text-stone-300 hover:bg-stone-800/60 hover:text-stone-100'}"
    style="padding-left: {depth * 14 + 6}px;"
    onclick={handleClick}
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

  {#if entry.is_dir && entry.isExpanded && entry.children}
    <div class="flex flex-col">
      {#each entry.children as child (child.path || child.relative_path)}
        <FileTreeNode
          entry={child}
          {activePath}
          depth={depth + 1}
          {onSelectFile}
          {onToggleDir}
        />
      {/each}
      {#if entry.children.length === 0}
        <div
          class="text-[10px] text-stone-500 italic py-1 font-mono"
          style="padding-left: {(depth + 1) * 14 + 20}px;"
        >
          (空のディレクトリ)
        </div>
      {/if}
    </div>
  {/if}
</div>
