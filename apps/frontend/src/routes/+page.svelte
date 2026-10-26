<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { Editor } from "@tiptap/core";
  import StarterKit from "@tiptap/starter-kit";
  import FileTreeNode, { type FileEntry } from "$lib/FileTreeNode.svelte";

  interface VaultEntry {
    name: string;
    path: string;
    last_opened: number;
  }

  interface VaultState {
    current_path: string;
    current_name: string;
    recent_vaults: VaultEntry[];
  }

  // State (Svelte 5 runes)
  let editorElement = $state<HTMLDivElement | null>(null);
  let editor = $state<Editor | null>(null);
  let editMode = $state<"hybrid" | "full">("hybrid"); // "hybrid" = Block Hybrid, "full" = Full Block / WYSIWYG
  let activeFile = $state<string>("readme.tmt");
  let activePath = $state<string>("tmtroot/readme.tmt");
  let isSaved = $state<boolean>(true);
  let isTauri = $state<boolean>(false);
  let statusMessage = $state<string>("");

  // Vault State
  let vaultName = $state<string>("telaion");
  let vaultPath = $state<string>(".");
  let recentVaults = $state<VaultEntry[]>([]);
  let isVaultMenuOpen = $state<boolean>(false);
  let customVaultInput = $state<string>("");

  // File Tree Explorer State
  let rootEntries = $state<FileEntry[]>([]);
  let expandedPaths = $state<Set<string>>(new Set());
  let isRefreshing = $state<boolean>(false);
  let unlistenWatcher: (() => void) | null = null;

  // Document metadata state
  let docMeta = $state<{ id: string; title: string; lang: string }>({
    id: "doc-telaion",
    title: "Telaion ワークスペース",
    lang: "ja",
  });

  const defaultContent = `
    <h1>=[ Telaion: 次世代 Tomet エディタ ]=</h1>
    <p>Telaion は <strong>Tomet</strong> 言語のためのワークスペース型デスクトップエディタです。</p>

    <div class="tomet-callout-card">
      <div class="text-xs font-semibold text-sky-400 uppercase tracking-wider mb-1">@callout(info)</div>
      <p class="m-0 text-stone-300">上部のトグルで <strong>「ブロックハイブリッド」</strong> と <strong>「フルブロック (WYSIWYG)」</strong> の 2 つの編集モードをシームレスに切り替えられます！</p>
    </div>

    <h2>==[ 主な特徴 ]==</h2>
    <ul>
      <li><strong>Vault 管理</strong>: 左上のセレクターから複数の Vault を瞬時に切り替え可能。</li>
      <li><strong>ブロックハイブリッド</strong>: コールアウトや @meta などのブロック要素はリッチ UI で操作し、テキストは記号を直接書けるハイブリッド体験。</li>
      <li><strong>フルブロック</strong>: 装飾記号を隠し、完全なリッチテキストとして見たまま執筆。</li>
      <li><strong>Tomet AST 連携</strong>: Rust バックエンドで直接 Tomet 構文と相互変換。</li>
    </ul>

    <h2>==[ コードブロック記法 ]==</h2>
    <pre><code>// Tomet のキー・バリュー記法
::meta{ id: sample-doc, author: "Tomet Team" }
=[ 見出し ]
本文テキスト。
</code></pre>
  `;

  onMount(async () => {
    isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

    if (!editorElement) return;

    editor = new Editor({
      element: editorElement,
      extensions: [
        StarterKit.configure({
          heading: {
            levels: [1, 2, 3, 4, 5, 6],
          },
        }),
      ],
      content: defaultContent,
      editorProps: {
        attributes: {
          class: "focus:outline-none min-h-[500px] text-stone-200 text-base leading-relaxed",
        },
      },
      onUpdate: () => {
        isSaved = false;
      },
    });

    // Load initial vault and workspace files via Tauri IPC
    if (isTauri) {
      await initVault();
      setupFileWatcher();
    } else {
      // Mock entries for browser dev
      rootEntries = [
        {
          name: "tmtroot",
          path: "tmtroot",
          relative_path: "tmtroot",
          is_dir: true,
          has_children: true,
          isExpanded: true,
          children: [
            { name: "readme.tmt", path: "tmtroot/readme.tmt", relative_path: "tmtroot/readme.tmt", is_dir: false, has_children: false },
            { name: "agents.tmt", path: "tmtroot/agents.tmt", relative_path: "tmtroot/agents.tmt", is_dir: false, has_children: false },
            { name: "contributing.tmt", path: "tmtroot/contributing.tmt", relative_path: "tmtroot/contributing.tmt", is_dir: false, has_children: false },
          ]
        },
        { name: "default.config.tmt", path: "default.config.tmt", relative_path: "default.config.tmt", is_dir: false, has_children: false },
      ];
    }

    // Register global keyboard shortcuts (Ctrl+S / Cmd+S)
    window.addEventListener("keydown", handleKeydown);
  });

  onDestroy(() => {
    if (typeof window !== "undefined") {
      window.removeEventListener("keydown", handleKeydown);
    }
    if (unlistenWatcher) {
      unlistenWatcher();
    }
    editor?.destroy();
  });

  function handleKeydown(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key === "s") {
      e.preventDefault();
      saveCurrentDocument();
    }
  }

  async function setupFileWatcher() {
    if (!isTauri) return;
    try {
      const { listen } = await import("@tauri-apps/api/event");
      unlistenWatcher = await listen<string[]>("workspace:changed", async (event) => {
        console.log("Workspace changed on disk:", event.payload);
        statusMessage = "外部変更を検知: エクスプローラーを更新中...";
        await refreshWorkspace();
        statusMessage = "エクスプローラー最新";
      });
    } catch (err: any) {
      console.warn("Watcher listener setup warning:", err);
    }
  }

  async function initVault() {
    if (!isTauri) return;
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const vState = await invoke<VaultState>("get_vault_state");
      if (vState) {
        vaultName = vState.current_name;
        vaultPath = vState.current_path;
        recentVaults = vState.recent_vaults;
      }
      await loadWorkspace();
    } catch (err: any) {
      console.error("Vault init error:", err);
      statusMessage = `Vault 初期化失敗: ${err?.message || err}`;
    }
  }

  async function switchVault(targetPath: string) {
    if (!isTauri || !targetPath.trim()) return;
    isVaultMenuOpen = false;
    statusMessage = `Vault 切り替え中: ${targetPath}...`;

    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const res = await invoke<{ state: VaultState; files: FileEntry[] }>("switch_vault", {
        path: targetPath,
      });

      vaultName = res.state.current_name;
      vaultPath = res.state.current_path;
      recentVaults = res.state.recent_vaults;
      expandedPaths.clear();
      rootEntries = res.files || [];
      customVaultInput = "";

      statusMessage = `Vault 開封: ${vaultName}`;

      // Automatically auto-expand tmtroot if present and open first .tmt
      await autoOpenInitialDocument(rootEntries);
    } catch (err: any) {
      console.error("Switch vault error:", err);
      statusMessage = `Vault 切り替えエラー: ${err?.message || err}`;
    }
  }

  async function loadWorkspace() {
    if (!isTauri) return;
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const entries = await invoke<FileEntry[]>("list_directory", { path: null });
      rootEntries = entries || [];
      await autoOpenInitialDocument(rootEntries);
    } catch (err: any) {
      console.error("Workspace load error:", err);
      statusMessage = `ファイル走査失敗: ${err?.message || err}`;
    }
  }

  async function autoOpenInitialDocument(entries: FileEntry[]) {
    // 1. If tmtroot exists, expand it automatically
    const tmtroot = entries.find(e => e.name === "tmtroot" && e.is_dir);
    if (tmtroot) {
      await toggleDirectory(tmtroot);
      const target = tmtroot.children?.find(c => c.name.includes("readme.tmt")) || tmtroot.children?.[0];
      if (target && !target.is_dir) {
        await openDocument(target.path || target.relative_path);
        return;
      }
    }

    // 2. Otherwise open first available .tmt in root
    const rootTmt = entries.find(e => e.name.endsWith(".tmt") && !e.is_dir);
    if (rootTmt) {
      await openDocument(rootTmt.path || rootTmt.relative_path);
    }
  }

  async function refreshWorkspace() {
    if (!isTauri || isRefreshing) return;
    isRefreshing = true;
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const entries = await invoke<FileEntry[]>("list_directory", { path: null });

      // Recursively re-expand paths that were already open
      await rehydrateExpanded(entries, expandedPaths);
      rootEntries = entries;
    } catch (err: any) {
      console.error("Failed to refresh workspace:", err);
    } finally {
      isRefreshing = false;
    }
  }

  async function rehydrateExpanded(entries: FileEntry[], openPaths: Set<string>) {
    const { invoke } = await import("@tauri-apps/api/core");
    for (const entry of entries) {
      if (entry.is_dir && openPaths.has(entry.relative_path)) {
        try {
          const children = await invoke<FileEntry[]>("list_directory", { path: entry.relative_path });
          entry.children = children;
          entry.isExpanded = true;
          if (children.length > 0) {
            await rehydrateExpanded(children, openPaths);
          }
        } catch (e) {
          console.error("Rehydrate error on", entry.relative_path, e);
        }
      }
    }
  }

  async function toggleDirectory(entry: FileEntry) {
    if (!entry.is_dir) return;

    if (entry.isExpanded) {
      entry.isExpanded = false;
      expandedPaths.delete(entry.relative_path);
      return;
    }

    // Lazy load children on demand
    entry.isLoading = true;
    if (!isTauri) {
      entry.isExpanded = true;
      entry.isLoading = false;
      expandedPaths.add(entry.relative_path);
      return;
    }

    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const children = await invoke<FileEntry[]>("list_directory", { path: entry.relative_path });
      entry.children = children || [];
      entry.isExpanded = true;
      expandedPaths.add(entry.relative_path);
    } catch (err: any) {
      console.error("Failed to list directory:", err);
      statusMessage = `ディレクトリ展開エラー: ${err?.message || err}`;
    } finally {
      entry.isLoading = false;
    }
  }

  async function handleFileSelect(file: FileEntry) {
    await openDocument(file.path || file.relative_path);
  }

  async function openDocument(filePath: string) {
    activePath = filePath;
    activeFile = filePath.split("/").pop() || filePath;
    statusMessage = `読み込み中: ${activeFile}...`;

    if (!isTauri || !editor) {
      statusMessage = `モック表示: ${activeFile}`;
      return;
    }

    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const pmDoc = await invoke<any>("read_tomet_document", { path: filePath });
      if (pmDoc) {
        editor.commands.setContent(pmDoc);
        isSaved = true;
        statusMessage = `ロード完了: ${activeFile}`;
      }
    } catch (err: any) {
      console.error("Failed to read Tomet document:", err);
      statusMessage = `読み込みエラー: ${err?.message || err}`;
    }
  }

  async function saveCurrentDocument() {
    if (!editor) return;
    statusMessage = "保存中...";

    if (!isTauri) {
      isSaved = true;
      statusMessage = "保存完了（ブラウザモック）";
      return;
    }

    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const json = editor.getJSON();
      await invoke("write_tomet_document", {
        path: activePath,
        doc: json,
      });
      isSaved = true;
      statusMessage = `保存完了: ${activePath}`;
    } catch (err: any) {
      console.error("Failed to save Tomet document:", err);
      statusMessage = `保存失敗: ${err?.message || err}`;
    }
  }

  function toggleMode(mode: "hybrid" | "full") {
    editMode = mode;
  }
</script>

<div class="flex h-screen w-screen overflow-hidden bg-stone-950 text-stone-200 select-none">
  <!-- Left Sidebar: Workspace Vault & Lazy Tree Explorer -->
  <aside class="w-68 border-r border-stone-800 bg-stone-900/60 flex flex-col shrink-0 relative">
    <!-- Vault Header & Switcher Trigger -->
    <div class="p-3 border-b border-stone-800 relative">
      <div class="relative group/vault">
        <button
          class="w-full flex items-center justify-between p-2 rounded-lg bg-stone-800/60 hover:bg-stone-800 transition border border-stone-700/50 text-left"
          onclick={() => isVaultMenuOpen = !isVaultMenuOpen}
          title="Vault を切り替える"
        >
          <div class="flex items-baseline gap-2 min-w-0 flex-1 mr-2">
            <span class="text-xs shrink-0">📂</span>
            <span class="font-bold text-xs text-stone-200 shrink-0">{vaultName}</span>
            <span class="text-[10px] text-stone-500 font-mono truncate flex-1">{vaultPath}</span>
          </div>
          <span class="text-stone-400 text-[10px] shrink-0">▼</span>
        </button>

        <!-- Hover detail tooltip showing full path with wrapping when menu is closed -->
        {#if !isVaultMenuOpen}
          <div
            class="absolute left-0 right-0 top-full mt-1.5 hidden group-hover/vault:block z-40 bg-stone-900 border border-stone-700/80 p-2 rounded-lg shadow-2xl text-xs pointer-events-none backdrop-blur-md"
          >
            <div class="flex items-center gap-1.5 font-semibold text-stone-200 mb-1">
              <span>📂</span>
              <span>{vaultName}</span>
            </div>
            <div class="text-[10px] text-stone-400 font-mono break-all leading-relaxed bg-stone-950/80 p-1.5 rounded border border-stone-800">
              {vaultPath}
            </div>
          </div>
        {/if}
      </div>

      <!-- Vault Switcher Dropdown Popover -->
      {#if isVaultMenuOpen}
        <div
          class="absolute top-full left-3 right-3 mt-1.5 bg-stone-900 border border-stone-700 rounded-lg shadow-2xl p-2 z-50 text-xs space-y-2 backdrop-blur-md"
        >
          <div class="px-2 py-1 text-[10px] font-semibold text-stone-400 uppercase tracking-wider flex items-center justify-between border-b border-stone-800 pb-1">
            <span>Recent Vaults</span>
            <button class="text-stone-400 hover:text-stone-200" onclick={() => isVaultMenuOpen = false}>✕</button>
          </div>

          <!-- Recent vaults list -->
          <div class="max-h-48 overflow-y-auto space-y-1">
            {#each recentVaults as vault}
              <button
                class="w-full text-left px-2 py-1.5 rounded transition group/vitem {vault.path === vaultPath ? 'bg-emerald-950/40 text-emerald-300 font-medium' : 'text-stone-300 hover:bg-stone-800'}"
                onclick={() => switchVault(vault.path)}
              >
                <div class="flex items-center justify-between gap-1">
                  <div class="flex items-baseline gap-2 min-w-0 flex-1">
                    <span class="font-medium text-xs text-stone-200 shrink-0">{vault.name}</span>
                    <span class="text-[10px] text-stone-500 font-mono truncate group-hover/vitem:hidden">{vault.path}</span>
                  </div>
                  {#if vault.path === vaultPath}
                    <span class="text-xs text-emerald-400 shrink-0">✓</span>
                  {/if}
                </div>
                <!-- Hover-expanded full path with wrapping -->
                <div class="hidden group-hover/vitem:block text-[10px] text-stone-400 font-mono break-all leading-relaxed bg-stone-950/60 p-1.5 rounded border border-stone-800/80 mt-1">
                  {vault.path}
                </div>
              </button>
            {/each}
          </div>

          <!-- Open other directory input -->
          <div class="pt-2 border-t border-stone-800 space-y-1.5">
            <span class="text-[10px] text-stone-400 block px-1">別の Vault パスを開く:</span>
            <div class="flex gap-1.5">
              <input
                type="text"
                placeholder="/path/to/vault"
                bind:value={customVaultInput}
                class="flex-1 bg-stone-950 border border-stone-700 rounded px-2 py-1 text-stone-200 font-mono text-[11px] focus:outline-none focus:border-stone-500"
                onkeydown={(e) => e.key === 'Enter' && switchVault(customVaultInput)}
              />
              <button
                class="px-2.5 py-1 bg-stone-800 hover:bg-stone-700 text-stone-200 rounded text-[11px] transition font-medium shrink-0"
                onclick={() => switchVault(customVaultInput)}
              >
                開く
              </button>
            </div>
          </div>
        </div>
      {/if}
    </div>

    <!-- Explorer Section Header -->
    <div class="px-3 pt-3 pb-1 flex items-center justify-between text-stone-400 border-b border-stone-800/50">
      <span class="text-[10px] font-bold tracking-wider uppercase text-stone-400">Explorer</span>
      <div class="flex items-center gap-1">
        <button
          class="p-1 rounded hover:bg-stone-800 text-stone-400 hover:text-stone-200 transition text-xs"
          onclick={refreshWorkspace}
          title="エクスプローラーを再読み込み"
        >
          <span class="inline-block {isRefreshing ? 'animate-spin' : ''}">🔄</span>
        </button>
      </div>
    </div>

    <!-- Lazy Loading Tree Explorer Area -->
    <div class="p-2 flex-1 overflow-y-auto space-y-0.5">
      {#if rootEntries.length === 0}
        <div class="text-stone-500 text-xs p-3 text-center italic font-mono">
          ファイルが見つかりません
        </div>
      {:else}
        {#each rootEntries as entry (entry.path || entry.relative_path)}
          <FileTreeNode
            {entry}
            {activePath}
            depth={0}
            onSelectFile={handleFileSelect}
            onToggleDir={toggleDirectory}
          />
        {/each}
      {/if}
    </div>

    <!-- Bottom Status / Diagnostics -->
    <div class="p-3 border-t border-stone-800/80 bg-stone-950/40 text-[11px] text-stone-400 space-y-1.5">
      <div class="flex items-center justify-between">
        <span>Backend:</span>
        <span class="font-mono text-emerald-400">{isTauri ? "Tauri v3 IPC" : "Web (Vite)"}</span>
      </div>
      {#if statusMessage}
        <div class="text-[10px] text-stone-400 truncate font-mono" title={statusMessage}>
          {statusMessage}
        </div>
      {/if}
    </div>
  </aside>

  <!-- Main Workspace Canvas -->
  <main class="flex-1 flex flex-col h-full overflow-hidden bg-stone-900/20">
    <!-- Top Bar: Navigation, Mode Switcher, Actions -->
    <header class="h-13 border-b border-stone-800/80 px-6 flex items-center justify-between shrink-0 bg-stone-900/40 backdrop-blur-sm">
      <!-- File info & Breadcrumb -->
      <div class="flex items-center gap-3">
        <span class="text-xs font-mono font-medium text-stone-300 bg-stone-800/60 px-2 py-1 rounded border border-stone-700/40">{activePath}</span>
        <span class="text-[11px] px-2 py-0.5 rounded-full {isSaved ? 'bg-emerald-950/60 text-emerald-400 border border-emerald-800/50' : 'bg-amber-950/60 text-amber-400 border border-amber-800/50'}">
          {isSaved ? "Saved" : "Modified"}
        </span>
      </div>

      <!-- Mode Switcher & Actions -->
      <div class="flex items-center gap-3">
        <!-- Dual Mode Switcher -->
        <div class="flex items-center bg-stone-950 p-0.5 rounded-lg border border-stone-800 text-xs">
          <button
            class="px-3 py-1.5 rounded-md transition flex items-center gap-1.5 {editMode === 'hybrid' ? 'bg-amber-500/20 text-amber-300 border border-amber-500/30 font-medium shadow-sm' : 'text-stone-400 hover:text-stone-200'}"
            onclick={() => toggleMode('hybrid')}
            title="ブロック要素はリッチUI、見出しやテキストはマークアップ構文の直感的なハイブリッド編集"
          >
            <span>⚡</span>
            <span>ブロックハイブリッド</span>
          </button>
          <button
            class="px-3 py-1.5 rounded-md transition flex items-center gap-1.5 {editMode === 'full' ? 'bg-sky-500/20 text-sky-300 border border-sky-500/30 font-medium shadow-sm' : 'text-stone-400 hover:text-stone-200'}"
            onclick={() => toggleMode('full')}
            title="装飾記号を隠し、完全なリッチテキストとして見たまま編集"
          >
            <span>🎨</span>
            <span>フルブロック (WYSIWYG)</span>
          </button>
        </div>

        <!-- Save Button -->
        <button
          class="px-3 py-1.5 rounded-md bg-stone-800 hover:bg-stone-700 text-stone-200 text-xs font-medium border border-stone-700/60 transition flex items-center gap-1.5"
          onclick={saveCurrentDocument}
          title="ドキュメントを保存 (Ctrl+S)"
        >
          <span>💾</span>
          <span>保存</span>
        </button>
      </div>
    </header>

    <!-- Editor Scroll Area with dynamic mode class -->
    <div class="flex-1 overflow-y-auto px-10 py-8 {editMode === 'hybrid' ? 'mode-hybrid' : 'mode-full'}">
      <div class="max-w-3xl mx-auto">
        <!-- Tomet Document Meta Inspector Card -->
        <div class="tomet-meta-card text-xs">
          <div class="flex items-center justify-between pb-2 mb-2 border-b border-stone-800 text-stone-400 font-semibold tracking-wider uppercase text-[10px]">
            <span>::meta (Document Metadata)</span>
            <span class="text-stone-400 font-mono">Tomet Header</span>
          </div>
          <div class="grid grid-cols-3 gap-3">
            <div>
              <span class="text-stone-400 block text-[10px] mb-0.5">id:</span>
              <input
                type="text"
                bind:value={docMeta.id}
                class="w-full bg-stone-900 border border-stone-800 rounded px-2 py-1 text-stone-200 font-mono text-xs focus:outline-none focus:border-stone-600"
              />
            </div>
            <div>
              <span class="text-stone-400 block text-[10px] mb-0.5">title:</span>
              <input
                type="text"
                bind:value={docMeta.title}
                class="w-full bg-stone-900 border border-stone-800 rounded px-2 py-1 text-stone-200 text-xs focus:outline-none focus:border-stone-600"
              />
            </div>
            <div>
              <span class="text-stone-400 block text-[10px] mb-0.5">lang:</span>
              <input
                type="text"
                bind:value={docMeta.lang}
                class="w-full bg-stone-900 border border-stone-800 rounded px-2 py-1 text-stone-200 font-mono text-xs focus:outline-none focus:border-stone-600"
              />
            </div>
          </div>
        </div>

        <!-- Mode Indicator Banner -->
        <div class="mb-4 px-3 py-1.5 rounded-md text-[11px] flex items-center gap-2 border {editMode === 'hybrid' ? 'bg-amber-950/20 text-amber-300 border-amber-900/40' : 'bg-sky-950/20 text-sky-300 border-sky-900/40'}">
          <span>{editMode === 'hybrid' ? '⚡' : '🎨'}</span>
          <span>
            {editMode === 'hybrid'
              ? 'ブロックハイブリッドモード: 見出しに Tomet 構文記号（=[ ]=）が美しく添えられ、構造を意識したハイブリッド執筆が可能です。'
              : 'フルブロックモード (WYSIWYG): 構文記号を完全に隠し、モダンなリッチテキストとして見たまま執筆できます。'}
          </span>
        </div>

        <!-- Tiptap Editor Canvas -->
        <div bind:this={editorElement} class="tiptap-editor cursor-text selection:bg-stone-800 select-text"></div>
      </div>
    </div>
  </main>
</div>
