<script lang="ts">
  // Quick open: type part of a name or path, choose a file. The matching and the order of the
  // results are ordiola's own; the palette owns the active row, the keys, the scrolling and the ARIA.
  import { CommandPalette, Kbd, type PaletteItem } from "@telaion/primitives";
  import { File, FileText, Settings } from "@telaion/primitives/icons";
  import type { FileEntry } from "$lib/types";

  const MAX_RESULTS = 30;

  let {
    open = $bindable(false),
    files,
    onopen,
  }: {
    open?: boolean;
    /** Every file of the workspace. */
    files: FileEntry[];
    /** The person chose a file; `path` is its path relative to the workspace. */
    onopen: (path: string) => void;
  } = $props();

  let query = $state("");

  // The name or the path contains what was typed, or the typed letters appear in the path in order.
  function matches(file: FileEntry, q: string): boolean {
    const path = file.relative_path.toLowerCase();
    if (file.name.toLowerCase().includes(q) || path.includes(q)) return true;
    let next = 0;
    for (let i = 0; i < path.length && next < q.length; i++) {
      if (path[i] === q[next]) next++;
    }
    return next === q.length;
  }

  const iconFor = (name: string) =>
    name.endsWith(".tmt") ? FileText : name.endsWith(".toml") ? Settings : File;

  const results = $derived.by(() => {
    const q = query.trim().toLowerCase();
    return (q ? files.filter((file) => matches(file, q)) : files).slice(0, MAX_RESULTS);
  });

  const items = $derived<(PaletteItem & { path: string })[]>(
    results.map((file) => {
      const path = file.relative_path || file.path;
      return { id: path, label: file.name, detail: file.relative_path, icon: iconFor(file.name), path };
    }),
  );
</script>

<CommandPalette
  bind:open
  bind:query
  {items}
  label="ファイルを開く"
  placeholder="ファイルを検索..."
  empty="一致するファイルが見つかりません"
  onselect={(item) => onopen(item.path)}
>
  {#snippet footer()}
    <span>{items.length} 件のファイル</span>
    <span><Kbd>↑↓</Kbd> 移動</span>
    <span><Kbd>↵</Kbd> 選択</span>
    <span><Kbd>Esc</Kbd> 閉じる</span>
  {/snippet}
</CommandPalette>
