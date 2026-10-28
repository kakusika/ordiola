<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { Editor } from "@tiptap/core";
  import StarterKit from "@tiptap/starter-kit";
  import FileTreeNode from "$lib/FileTreeNode.svelte";
  import CodeEditor from "$lib/CodeEditor.svelte";
  import type {
    FileEntry,
    VaultEntry,
    VaultState,
    InlineAction,
    ContextMenuState,
    DeleteConfirmState,
  } from "$lib/types";

  // State (Svelte 5 runes)
  let editorElement = $state<HTMLDivElement | null>(null);
  let editor = $state<Editor | null>(null);
  let editMode = $state<"source" | "hybrid" | "full">("source"); // "source" = CodeMirror, "hybrid" = Block Hybrid, "full" = Full Block / WYSIWYG
  let rawTextContent = $state<string>("");
  let activeFile = $state<string>("readme.tmt");
  let activePath = $state<string>("tmtroot/readme.tmt");
  let isSaved = $state<boolean>(true);
  let isTauri = $state<boolean>(false);
  let statusMessage = $state<string>("");

  let isTmtFile = $derived(activeFile.endsWith(".tmt"));

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

  let selectedEntry = $state<FileEntry | null>(null);
  let selectedPath = $derived<string | null>(
    selectedEntry ? selectedEntry.relative_path || selectedEntry.path : null
  );
  let selectedDir = $derived.by(() => {
    if (!selectedEntry) return "";
    if (selectedEntry.is_dir) return selectedEntry.relative_path || selectedEntry.path;
    const p = selectedEntry.relative_path || selectedEntry.path;
    const idx = p.lastIndexOf("/");
    return idx > 0 ? p.substring(0, idx) : "";
  });

  let inlineAction = $state<InlineAction | null>(null);
  let rootNewName = $state<string>("");
  let isRootCreating = $derived(
    inlineAction !== null &&
      (inlineAction.type === "new-file" || inlineAction.type === "new-folder") &&
      (inlineAction.parentPath === "" || inlineAction.parentPath === ".")
  );

  let contextMenu = $state<ContextMenuState | null>(null);
  let deleteConfirmModal = $state<DeleteConfirmState | null>(null);

  // Document metadata state
  let docMeta = $state<{ id: string; title: string; lang: string }>({
    id: "doc-telaion",
    title: "Telaion ワークスペース",
    lang: "ja",
  });

  const defaultTometContent = `::config(
  export: {
    type: commonmark,
    path: "README.md"
  }
)
::meta{
  id: doc-readme,
  lang: ja
}

=[ Telaion: 次世代 Tomet エディタ ]=

Telaion は Tomet 言語のためのワークスペース型デスクトップエディタです。

==[ 主な特徴 ]==

- Vault 管理: 複数のワークスペースを瞬時に切り替え
- ソース編集: CodeMirror 6 による安全・快適な生テキスト編集
- ブロックハイブリッド: ブロック要素とマークアップのハイブリッド体験
- フルブロック (WYSIWYG): 装飾記号を隠したリッチテキスト執筆
`;

  onMount(async () => {
    isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

    // Initialize Tiptap instance for hybrid/full modes
    if (editorElement) {
      editor = new Editor({
        element: editorElement,
        extensions: [
          StarterKit.configure({
            heading: {
              levels: [1, 2, 3, 4, 5, 6],
            },
          }),
        ],
        content: `<h1>=[ Telaion: 次世代 Tomet エディタ ]=</h1><p>Telaion は <strong>Tomet</strong> 言語のためのワークスペース型デスクトップエディタです。</p>`,
        editorProps: {
          attributes: {
            class: "focus:outline-none min-h-[500px] text-stone-200 text-base leading-relaxed",
          },
        },
        onUpdate: () => {
          if (editMode !== "source") {
            isSaved = false;
          }
        },
      });
    }

    // Load initial vault and workspace files via Tauri IPC
    if (isTauri) {
      await initVault();
      setupFileWatcher();
    } else {
      rawTextContent = defaultTometContent;
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
        { name: "Cargo.toml", path: "Cargo.toml", relative_path: "Cargo.toml", is_dir: false, has_children: false },
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
      return;
    }

    if (e.key === "Escape") {
      if (deleteConfirmModal) {
        deleteConfirmModal = null;
        return;
      }
      if (contextMenu) {
        closeContextMenu();
        return;
      }
      if (inlineAction) {
        cancelInlineAction();
        return;
      }
    }

    // Explorer shortcuts when not typing in an input or contenteditable editor
    const targetTag = (e.target as HTMLElement)?.tagName?.toLowerCase();
    const isEditing =
      targetTag === "input" ||
      targetTag === "textarea" ||
      (e.target as HTMLElement)?.isContentEditable;

    if (!isEditing && selectedEntry) {
      if (e.key === "F2") {
        e.preventDefault();
        startRename(selectedEntry);
      } else if (e.key === "Delete") {
        e.preventDefault();
        confirmDelete(selectedEntry);
      }
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

      // Automatically auto-expand tmtroot if present and open first file
      await autoOpenInitialDocument(rootEntries);
    } catch (err: any) {
      console.error("Switch vault error:", err);
      statusMessage = `Vault 切り替えエラー: ${err?.message || err}`;
    }
  }

  async function openFolderDialog() {
    isVaultMenuOpen = false;
    if (!isTauri) {
      statusMessage = "ブラウザ環境ではネイティブフォルダ選択は利用できません";
      return;
    }
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const selected = await invoke<string | null>("pick_vault_directory");
      if (selected) {
        await switchVault(selected);
      }
    } catch (err: any) {
      console.error("Folder picker error:", err);
      statusMessage = `フォルダ選択失敗: ${err?.message || err}`;
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
    const tmtroot = entries.find((e) => e.name === "tmtroot" && e.is_dir);
    if (tmtroot) {
      await toggleDirectory(tmtroot);
      const target =
        tmtroot.children?.find((c: FileEntry) => c.name.includes("readme.tmt")) ||
        tmtroot.children?.[0];
      if (target && !target.is_dir) {
        await openDocument(target.path || target.relative_path);
        return;
      }
    }

    // 2. Otherwise open first available .tmt in root
    const rootTmt = entries.find((e) => e.name.endsWith(".tmt") && !e.is_dir);
    if (rootTmt) {
      await openDocument(rootTmt.path || rootTmt.relative_path);
      return;
    }

    // 3. Fallback to any text file
    const anyFile = entries.find((e) => !e.is_dir);
    if (anyFile) {
      await openDocument(anyFile.path || anyFile.relative_path);
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
    selectedEntry = file;
    await openDocument(file.path || file.relative_path);
  }

  function startCreateFile(parentDir: string = "") {
    inlineAction = {
      type: "new-file",
      parentPath: parentDir,
    };
    rootNewName = "";
    if (parentDir) {
      expandPathRecursive(parentDir);
    }
  }

  function startCreateFolder(parentDir: string = "") {
    inlineAction = {
      type: "new-folder",
      parentPath: parentDir,
    };
    rootNewName = "";
    if (parentDir) {
      expandPathRecursive(parentDir);
    }
  }

  function startRename(entry: FileEntry) {
    inlineAction = {
      type: "rename",
      parentPath: "",
      targetEntry: entry,
    };
  }

  function confirmDelete(entry: FileEntry) {
    deleteConfirmModal = {
      visible: true,
      entry,
    };
  }

  function cancelInlineAction() {
    inlineAction = null;
    rootNewName = "";
  }

  function closeContextMenu() {
    contextMenu = null;
  }

  function handleContextMenu(e: MouseEvent, entry: FileEntry) {
    selectedEntry = entry;
    const x = Math.min(e.clientX, window.innerWidth - 210);
    const y = Math.min(e.clientY, window.innerHeight - 230);
    contextMenu = {
      visible: true,
      x: Math.max(10, x),
      y: Math.max(10, y),
      entry,
    };
  }

  function handleBackgroundContextMenu(e: MouseEvent) {
    if (e.defaultPrevented) return;
    e.preventDefault();
    const x = Math.min(e.clientX, window.innerWidth - 210);
    const y = Math.min(e.clientY, window.innerHeight - 210);
    contextMenu = {
      visible: true,
      x: Math.max(10, x),
      y: Math.max(10, y),
      entry: null,
    };
  }

  function collapseAll() {
    expandedPaths.clear();
    function collapseInTree(entries: FileEntry[]) {
      for (const e of entries) {
        if (e.is_dir) {
          e.isExpanded = false;
          if (e.children) collapseInTree(e.children);
        }
      }
    }
    collapseInTree(rootEntries);
  }

  async function expandPathRecursive(dirPath: string) {
    expandedPaths.add(dirPath);
    async function findAndExpand(entries: FileEntry[]): Promise<boolean> {
      for (const e of entries) {
        if (e.is_dir && (e.relative_path === dirPath || e.path === dirPath)) {
          if (!e.children || e.children.length === 0) {
            await toggleDirectory(e);
          } else {
            e.isExpanded = true;
          }
          return true;
        }
        if (e.is_dir && e.children) {
          const found = await findAndExpand(e.children);
          if (found) {
            e.isExpanded = true;
            expandedPaths.add(e.relative_path || e.path);
            return true;
          }
        }
      }
      return false;
    }
    await findAndExpand(rootEntries);
  }

  async function commitInlineAction(value: string) {
    if (!inlineAction) return;
    const action = inlineAction;
    inlineAction = null;
    rootNewName = "";

    const targetRelative = action.parentPath
      ? `${action.parentPath}/${value}`
      : value;

    if (!isTauri) {
      if (action.type === "new-file") {
        rootEntries.push({
          name: value,
          path: targetRelative,
          relative_path: targetRelative,
          is_dir: false,
          has_children: false,
        });
        await openDocument(targetRelative);
      } else if (action.type === "new-folder") {
        rootEntries.push({
          name: value,
          path: targetRelative,
          relative_path: targetRelative,
          is_dir: true,
          has_children: false,
          children: [],
        });
      } else if (action.type === "rename" && action.targetEntry) {
        action.targetEntry.name = value;
        action.targetEntry.relative_path = targetRelative;
        action.targetEntry.path = targetRelative;
      }
      return;
    }

    try {
      const { invoke } = await import("@tauri-apps/api/core");
      if (action.type === "new-file") {
        const initialContent = targetRelative.endsWith(".tmt")
          ? `=[ ${value.replace(/\.tmt$/, "")} ]=\n\n`
          : "";
        await invoke<FileEntry>("create_file", {
          path: targetRelative,
          content: initialContent,
        });
        if (action.parentPath) {
          expandedPaths.add(action.parentPath);
        }
        await refreshWorkspace();
        await openDocument(targetRelative);
        statusMessage = `ファイル作成: ${targetRelative}`;
      } else if (action.type === "new-folder") {
        await invoke<FileEntry>("create_directory", {
          path: targetRelative,
        });
        expandedPaths.add(targetRelative);
        if (action.parentPath) {
          expandedPaths.add(action.parentPath);
        }
        await refreshWorkspace();
        statusMessage = `フォルダ作成: ${targetRelative}`;
      } else if (action.type === "rename" && action.targetEntry) {
        const oldRel = action.targetEntry.relative_path || action.targetEntry.path;
        await invoke<FileEntry>("rename_entry", {
          oldPath: oldRel,
          newName: value,
        });
        if (activePath === oldRel || activePath === action.targetEntry.path) {
          const parent = action.targetEntry.relative_path.includes("/")
            ? action.targetEntry.relative_path.substring(0, action.targetEntry.relative_path.lastIndexOf("/"))
            : "";
          const newRel = parent ? `${parent}/${value}` : value;
          activePath = newRel;
          activeFile = value;
        }
        await refreshWorkspace();
        statusMessage = `名前変更: ${value}`;
      }
    } catch (err: any) {
      console.error("File operation error:", err);
      statusMessage = `操作失敗: ${err?.message || err}`;
    }
  }

  async function executeDelete() {
    if (!deleteConfirmModal) return;
    const target = deleteConfirmModal.entry;
    deleteConfirmModal = null;

    const targetPath = target.relative_path || target.path;

    if (!isTauri) {
      rootEntries = rootEntries.filter((e) => e.relative_path !== targetPath);
      statusMessage = `削除完了（モック）: ${target.name}`;
      return;
    }

    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("delete_entry", { path: targetPath });

      if (activePath === targetPath || activePath.startsWith(targetPath + "/")) {
        rawTextContent = "";
        activePath = "";
        activeFile = "";
        isSaved = true;
      }

      await refreshWorkspace();
      statusMessage = `削除完了: ${target.name}`;
    } catch (err: any) {
      console.error("Delete error:", err);
      statusMessage = `削除失敗: ${err?.message || err}`;
    }
  }

  async function handleRootNewKeyDown(e: KeyboardEvent) {
    if (e.key === "Enter") {
      e.preventDefault();
      e.stopPropagation();
      const val = rootNewName.trim();
      if (val) {
        await commitInlineAction(val);
      } else {
        cancelInlineAction();
      }
    } else if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      cancelInlineAction();
    }
  }

  function focusRootInput(node: HTMLInputElement) {
    node.focus();
  }

  async function openDocument(filePath: string) {
    activePath = filePath;
    activeFile = filePath.split("/").pop() || filePath;
    statusMessage = `読み込み中: ${activeFile}...`;

    if (!isTauri) {
      statusMessage = `モック表示: ${activeFile}`;
      if (activeFile.endsWith(".tmt")) {
        rawTextContent = defaultTometContent;
      } else {
        rawTextContent = `// Content of ${activeFile}\n`;
      }
      isSaved = true;
      return;
    }

    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const text = await invoke<string>("read_text_file", { path: filePath });
      rawTextContent = text;
      isSaved = true;

      // If opening non-Tomet file, force source mode
      if (!filePath.endsWith(".tmt")) {
        editMode = "source";
        statusMessage = `ロード完了: ${activeFile}`;
        return;
      }

      // If opening .tmt file in hybrid/full mode, parse into Tiptap
      if (editMode === "hybrid" || editMode === "full") {
        try {
          const pmDoc = await invoke<any>("parse_tomet_string", { source: text });
          if (pmDoc && editor) {
            editor.commands.setContent(pmDoc);
          }
        } catch (e: any) {
          console.warn("Tomet parse warning, falling back to source mode:", e);
          editMode = "source";
          statusMessage = `構文解析注意（ソース編集で表示）: ${e?.message || e}`;
          return;
        }
      }

      statusMessage = `ロード完了: ${activeFile}`;
    } catch (err: any) {
      console.error("Failed to read file:", err);
      statusMessage = `読み込みエラー: ${err?.message || err}`;
    }
  }

  async function saveCurrentDocument() {
    statusMessage = "保存中...";

    if (!isTauri) {
      isSaved = true;
      statusMessage = "保存完了（ブラウザモック）";
      return;
    }

    try {
      const { invoke } = await import("@tauri-apps/api/core");
      if (editMode === "source" || !isTmtFile) {
        await invoke("write_text_file", {
          path: activePath,
          content: rawTextContent,
        });
      } else {
        const json = editor?.getJSON();
        if (json) {
          const tometStr = await invoke<string>("serialize_prosemirror_doc", { doc: json });
          await invoke("write_text_file", {
            path: activePath,
            content: tometStr,
          });
          rawTextContent = tometStr;
        }
      }
      isSaved = true;
      statusMessage = `保存完了: ${activePath}`;
    } catch (err: any) {
      console.error("Failed to save document:", err);
      statusMessage = `保存失敗: ${err?.message || err}`;
    }
  }

  function handleSourceChange(newContent: string) {
    rawTextContent = newContent;
    isSaved = false;
  }

  async function toggleMode(mode: "source" | "hybrid" | "full") {
    if (mode === editMode) return;

    if (!isTmtFile && mode !== "source") {
      statusMessage = "Tomet ファイル (.tmt) 以外はソース編集モードのみ対応しています";
      return;
    }

    if (mode === "hybrid" || mode === "full") {
      if (isTauri && editor) {
        try {
          const { invoke } = await import("@tauri-apps/api/core");
          const pmDoc = await invoke<any>("parse_tomet_string", { source: rawTextContent });
          if (pmDoc) {
            editor.commands.setContent(pmDoc);
          }
        } catch (err: any) {
          console.warn("Could not parse Tomet into ProseMirror:", err);
          statusMessage = `構文解析エラーのためソース編集のままにします: ${err?.message || err}`;
          return;
        }
      }
    } else if (mode === "source" && (editMode === "hybrid" || editMode === "full")) {
      if (isTauri && editor) {
        try {
          const { invoke } = await import("@tauri-apps/api/core");
          const json = editor.getJSON();
          const tometStr = await invoke<string>("serialize_prosemirror_doc", { doc: json });
          if (tometStr) {
            rawTextContent = tometStr;
          }
        } catch (err: any) {
          console.warn("Could not serialize ProseMirror into Tomet:", err);
        }
      }
    }

    editMode = mode;
  }
</script>

<div class="flex h-screen w-screen overflow-hidden bg-stone-950 text-stone-200 select-none">
  <!-- Left Sidebar: Workspace Vault & Lazy Tree Explorer -->
  <aside class="w-68 border-r border-stone-800 bg-stone-900/60 flex flex-col shrink-0 relative">
    <!-- Vault Header & Switcher Trigger -->
    <div class="p-3 border-b border-stone-800 relative">
      <div class="flex items-center gap-1.5 relative group/vault">
        <button
          class="flex-1 flex items-center justify-between p-2 rounded-lg bg-stone-800/60 hover:bg-stone-800 transition border border-stone-700/50 text-left cursor-pointer min-w-0"
          onclick={() => (isVaultMenuOpen = !isVaultMenuOpen)}
          title="Vault を切り替える"
        >
          <div class="flex items-baseline gap-2 min-w-0 flex-1 mr-2">
            <span class="text-xs shrink-0">📂</span>
            <span class="font-bold text-xs text-stone-200 shrink-0">{vaultName}</span>
            <span class="text-[10px] text-stone-500 font-mono truncate flex-1">{vaultPath}</span>
          </div>
          <span class="text-stone-400 text-[10px] shrink-0">▼</span>
        </button>

        <!-- Quick Native Folder Picker Trigger -->
        <button
          class="p-2 rounded-lg bg-stone-800/60 hover:bg-stone-800 hover:text-sky-300 transition border border-stone-700/50 text-stone-400 text-xs cursor-pointer shrink-0"
          onclick={openFolderDialog}
          title="ネイティブフォルダ選択でVaultを開く"
        >
          <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/>
            <line x1="12" y1="11" x2="12" y2="17"/>
            <line x1="9" y1="14" x2="15" y2="14"/>
          </svg>
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
            <button class="text-stone-400 hover:text-stone-200 cursor-pointer" onclick={() => (isVaultMenuOpen = false)}>✕</button>
          </div>

          <!-- Recent vaults list -->
          <div class="max-h-48 overflow-y-auto space-y-1">
            {#each recentVaults as vault}
              <button
                class="w-full text-left px-2 py-1.5 rounded transition group/vitem cursor-pointer {vault.path === vaultPath ? 'bg-emerald-950/40 text-emerald-300 font-medium' : 'text-stone-300 hover:bg-stone-800'}"
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

          <!-- Native Directory Dialog Button -->
          <div class="pt-2 border-t border-stone-800">
            <button
              class="w-full flex items-center justify-center gap-2 px-3 py-2 rounded-md bg-sky-950/40 hover:bg-sky-900/60 text-sky-300 hover:text-sky-100 border border-sky-500/30 hover:border-sky-500/60 transition font-medium text-xs cursor-pointer shadow-sm"
              onclick={openFolderDialog}
            >
              <span>📂</span>
              <span>フォルダを選択して開く...</span>
            </button>
          </div>

          <!-- Open other directory input -->
          <div class="pt-2 border-t border-stone-800 space-y-1.5">
            <span class="text-[10px] text-stone-400 block px-1">直接パスを入力:</span>
            <div class="flex gap-1.5">
              <input
                type="text"
                placeholder="/path/to/vault"
                bind:value={customVaultInput}
                class="flex-1 bg-stone-950 border border-stone-700 rounded px-2 py-1 text-stone-200 font-mono text-[11px] focus:outline-none focus:border-stone-500"
                onkeydown={(e) => e.key === "Enter" && switchVault(customVaultInput)}
              />
              <button
                class="px-2.5 py-1 bg-stone-800 hover:bg-stone-700 text-stone-200 rounded text-[11px] transition font-medium shrink-0 cursor-pointer"
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
      <div class="flex items-center gap-0.5">
        <button
          class="p-1 rounded hover:bg-stone-800 text-stone-400 hover:text-stone-200 transition text-xs cursor-pointer"
          onclick={() => startCreateFile(selectedDir || "")}
          title="新しいファイルを作成"
        >
          <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
            <polyline points="14 2 14 8 20 8"/>
            <line x1="12" y1="18" x2="12" y2="12"/>
            <line x1="9" y1="15" x2="15" y2="15"/>
          </svg>
        </button>
        <button
          class="p-1 rounded hover:bg-stone-800 text-stone-400 hover:text-stone-200 transition text-xs cursor-pointer"
          onclick={() => startCreateFolder(selectedDir || "")}
          title="新しいフォルダを作成"
        >
          <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/>
            <line x1="12" y1="11" x2="12" y2="17"/>
            <line x1="9" y1="14" x2="15" y2="14"/>
          </svg>
        </button>
        <button
          class="p-1 rounded hover:bg-stone-800 text-stone-400 hover:text-stone-200 transition text-xs cursor-pointer"
          onclick={refreshWorkspace}
          title="エクスプローラーを再読み込み"
        >
          <svg class="w-3.5 h-3.5 {isRefreshing ? 'animate-spin' : ''}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M21.5 2v6h-6M2.5 22v-6h6M2 11.5a10 10 0 0 1 18.8-4.3M22 12.5a10 10 0 0 1-18.8 4.3"/>
          </svg>
        </button>
        <button
          class="p-1 rounded hover:bg-stone-800 text-stone-400 hover:text-stone-200 transition text-xs cursor-pointer"
          onclick={collapseAll}
          title="すべてのフォルダを折りたたむ"
        >
          <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="4 14 10 14 10 20"/>
            <polyline points="20 10 14 10 14 4"/>
            <line x1="14" y1="10" x2="21" y2="3"/>
            <line x1="3" y1="21" x2="10" y2="14"/>
          </svg>
        </button>
      </div>
    </div>

    <!-- Lazy Loading Tree Explorer Area -->
    <div
      class="p-2 flex-1 overflow-y-auto space-y-0.5"
      role="tree"
      tabindex="0"
      onclick={() => (selectedEntry = null)}
      onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') selectedEntry = null; }}
      oncontextmenu={handleBackgroundContextMenu}
    >
      <!-- Root-level inline create row -->
      {#if isRootCreating}
        <div
          class="flex items-center gap-1.5 py-1 px-1.5 rounded bg-stone-900 border border-sky-500 shadow-sm mb-1"
          style="padding-left: 6px;"
          role="presentation"
          onclick={(e) => e.stopPropagation()}
          onkeydown={(e) => e.stopPropagation()}
        >
          <span class="w-3 shrink-0"></span>
          <span class="text-sm shrink-0">
            {inlineAction?.type === "new-folder" ? "📁" : "📄"}
          </span>
          <input
            type="text"
            bind:value={rootNewName}
            placeholder={inlineAction?.type === "new-folder" ? "フォルダ名" : "ファイル名.tmt"}
            class="flex-1 bg-stone-950 border border-sky-500 rounded px-1.5 py-0.5 text-[11px] font-mono text-stone-100 focus:outline-none"
            onkeydown={handleRootNewKeyDown}
            onblur={() => setTimeout(() => { if (isRootCreating) cancelInlineAction(); }, 150)}
            use:focusRootInput
          />
        </div>
      {/if}

      {#if rootEntries.length === 0 && !isRootCreating}
        <div class="text-stone-500 text-xs p-3 text-center italic font-mono">
          ファイルが見つかりません
        </div>
      {:else}
        {#each rootEntries as entry (entry.path || entry.relative_path)}
          <FileTreeNode
            {entry}
            {activePath}
            {selectedPath}
            depth={0}
            {inlineAction}
            onSelectFile={handleFileSelect}
            onSelectEntry={(ent) => (selectedEntry = ent)}
            onToggleDir={toggleDirectory}
            onContextMenu={handleContextMenu}
            onCommitInlineAction={commitInlineAction}
            onCancelInlineAction={cancelInlineAction}
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
        <span class="text-xs font-mono font-medium text-stone-300 bg-stone-800/60 px-2.5 py-1 rounded border border-stone-700/40 flex items-center gap-1.5">
          <span>{isTmtFile ? "📜" : "📄"}</span>
          <span>{activePath}</span>
        </span>
        <span class="text-[11px] px-2 py-0.5 rounded-full {isSaved ? 'bg-emerald-950/60 text-emerald-400 border border-emerald-800/50' : 'bg-amber-950/60 text-amber-400 border border-amber-800/50'}">
          {isSaved ? "Saved" : "Modified"}
        </span>
      </div>

      <!-- Mode Switcher & Actions -->
      <div class="flex items-center gap-3">
        <!-- Mode Switcher -->
        <div class="flex items-center bg-stone-950 p-0.5 rounded-lg border border-stone-800 text-xs">
          <!-- Source Editor Button (Available for all text files) -->
          <button
            class="px-3 py-1.5 rounded-md transition flex items-center gap-1.5 cursor-pointer {editMode === 'source' ? 'bg-sky-500/20 text-sky-300 border border-sky-500/30 font-medium shadow-sm' : 'text-stone-400 hover:text-stone-200'}"
            onclick={() => toggleMode('source')}
            title="ソース編集: 生テキストを直接編集 (行番号・等幅・高信頼性)"
          >
            <span>📄</span>
            <span>ソース編集</span>
          </button>

          {#if isTmtFile}
            <button
              class="px-3 py-1.5 rounded-md transition flex items-center gap-1.5 cursor-pointer {editMode === 'hybrid' ? 'bg-amber-500/20 text-amber-300 border border-amber-500/30 font-medium shadow-sm' : 'text-stone-400 hover:text-stone-200'}"
              onclick={() => toggleMode('hybrid')}
              title="ブロックハイブリッド: ブロック要素はリッチUI、見出しやテキストはマークアップ構文の直感的なハイブリッド編集"
            >
              <span>⚡</span>
              <span>ハイブリッド</span>
            </button>
            <button
              class="px-3 py-1.5 rounded-md transition flex items-center gap-1.5 cursor-pointer {editMode === 'full' ? 'bg-emerald-500/20 text-emerald-300 border border-emerald-500/30 font-medium shadow-sm' : 'text-stone-400 hover:text-stone-200'}"
              onclick={() => toggleMode('full')}
              title="フルブロック (WYSIWYG): 装飾記号を隠し、完全なリッチテキストとして見たまま編集"
            >
              <span>🎨</span>
              <span>フルブロック</span>
            </button>
          {:else}
            <span class="px-2 py-1 text-[10px] text-stone-500 font-mono select-none">
              プレーンテキスト
            </span>
          {/if}
        </div>

        <!-- Save Button -->
        <button
          class="px-3 py-1.5 rounded-md bg-stone-800 hover:bg-stone-700 text-stone-200 text-xs font-medium border border-stone-700/60 transition flex items-center gap-1.5 cursor-pointer"
          onclick={saveCurrentDocument}
          title="ドキュメントを保存 (Ctrl+S)"
        >
          <span>💾</span>
          <span>保存</span>
        </button>
      </div>
    </header>

    <!-- CodeMirror 6 Source Text Editor (Active when editMode === 'source') -->
    <div class="flex-1 overflow-hidden relative" class:hidden={editMode !== 'source'}>
      <CodeEditor
        content={rawTextContent}
        filePath={activePath}
        onChange={handleSourceChange}
        onSave={saveCurrentDocument}
      />
    </div>

    <!-- Tiptap WYSIWYG / Hybrid Editor (Active when editMode !== 'source') -->
    <div
      class="flex-1 overflow-y-auto px-10 py-8 {editMode === 'hybrid' ? 'mode-hybrid' : 'mode-full'}"
      class:hidden={editMode === 'source'}
    >
      <div class="max-w-3xl mx-auto">
        <!-- Tomet Document Meta Inspector Card -->
        {#if isTmtFile}
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
        {/if}

        <!-- Mode Indicator Banner -->
        <div class="mb-4 px-3 py-1.5 rounded-md text-[11px] flex items-center gap-2 border {editMode === 'hybrid' ? 'bg-amber-950/20 text-amber-300 border-amber-900/40' : 'bg-emerald-950/20 text-emerald-300 border-emerald-900/40'}">
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

  <!-- Context Menu Overlay -->
  {#if contextMenu && contextMenu.visible}
    <!-- Invisible full screen backdrop to dismiss menu on click -->
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="fixed inset-0 z-50 cursor-default"
      onclick={closeContextMenu}
      oncontextmenu={(e) => {
        e.preventDefault();
        closeContextMenu();
      }}
    ></div>

    <!-- Popover -->
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="fixed z-50 min-w-44 bg-stone-900/95 border border-stone-700/80 rounded-lg shadow-2xl py-1 text-xs text-stone-200 backdrop-blur-md"
      style="left: {contextMenu.x}px; top: {contextMenu.y}px;"
      onclick={(e) => e.stopPropagation()}
    >
      {#if contextMenu.entry}
        <div class="px-2.5 py-1 text-[10px] font-mono text-stone-400 truncate border-b border-stone-800 mb-1 max-w-56">
          {contextMenu.entry.name}
        </div>

        {#if contextMenu.entry.is_dir}
          <button
            class="w-full flex items-center gap-2 px-2.5 py-1.5 hover:bg-stone-800 text-stone-200 hover:text-stone-100 transition text-left cursor-pointer"
            onclick={() => {
              const p = contextMenu?.entry?.relative_path || "";
              closeContextMenu();
              startCreateFile(p);
            }}
          >
            <span class="text-xs">📄</span>
            <span>新しいファイル...</span>
          </button>
          <button
            class="w-full flex items-center gap-2 px-2.5 py-1.5 hover:bg-stone-800 text-stone-200 hover:text-stone-100 transition text-left cursor-pointer"
            onclick={() => {
              const p = contextMenu?.entry?.relative_path || "";
              closeContextMenu();
              startCreateFolder(p);
            }}
          >
            <span class="text-xs">📁</span>
            <span>新しいフォルダ...</span>
          </button>
          <div class="my-1 border-t border-stone-800"></div>
        {/if}

        <button
          class="w-full flex items-center gap-2 px-2.5 py-1.5 hover:bg-stone-800 text-stone-200 hover:text-stone-100 transition text-left cursor-pointer"
          onclick={() => {
            const ent = contextMenu?.entry;
            closeContextMenu();
            if (ent) startRename(ent);
          }}
        >
          <span class="text-xs">✏️</span>
          <span>名前の変更 (F2)</span>
        </button>

        <div class="my-1 border-t border-stone-800"></div>

        <button
          class="w-full flex items-center gap-2 px-2.5 py-1.5 hover:bg-red-950/40 text-red-400 hover:text-red-300 transition text-left cursor-pointer"
          onclick={() => {
            const ent = contextMenu?.entry;
            closeContextMenu();
            if (ent) confirmDelete(ent);
          }}
        >
          <span class="text-xs">🗑️</span>
          <span>削除... (Delete)</span>
        </button>
      {:else}
        <button
          class="w-full flex items-center gap-2 px-2.5 py-1.5 hover:bg-stone-800 text-stone-200 hover:text-stone-100 transition text-left cursor-pointer"
          onclick={() => {
            closeContextMenu();
            startCreateFile("");
          }}
        >
          <span class="text-xs">📄</span>
          <span>新しいファイル...</span>
        </button>
        <button
          class="w-full flex items-center gap-2 px-2.5 py-1.5 hover:bg-stone-800 text-stone-200 hover:text-stone-100 transition text-left cursor-pointer"
          onclick={() => {
            closeContextMenu();
            startCreateFolder("");
          }}
        >
          <span class="text-xs">📁</span>
          <span>新しいフォルダ...</span>
        </button>
        <div class="my-1 border-t border-stone-800"></div>
        <button
          class="w-full flex items-center gap-2 px-2.5 py-1.5 hover:bg-stone-800 text-stone-200 hover:text-stone-100 transition text-left cursor-pointer"
          onclick={() => {
            closeContextMenu();
            collapseAll();
          }}
        >
          <span class="text-xs">📂</span>
          <span>すべて折りたたむ</span>
        </button>
        <button
          class="w-full flex items-center gap-2 px-2.5 py-1.5 hover:bg-stone-800 text-stone-200 hover:text-stone-100 transition text-left cursor-pointer"
          onclick={() => {
            closeContextMenu();
            refreshWorkspace();
          }}
        >
          <span class="text-xs">🔄</span>
          <span>エクスプローラー更新</span>
        </button>
      {/if}
    </div>
  {/if}

  <!-- Delete Confirmation Modal -->
  {#if deleteConfirmModal && deleteConfirmModal.visible}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="fixed inset-0 z-50 bg-black/60 backdrop-blur-xs flex items-center justify-center p-4"
      onclick={() => (deleteConfirmModal = null)}
    >
      <div
        class="bg-stone-900 border border-stone-700 rounded-xl shadow-2xl p-5 max-w-sm w-full text-xs space-y-3"
        onclick={(e) => e.stopPropagation()}
      >
        <div class="flex items-start gap-3">
          <div class="p-2 rounded-lg bg-red-950/60 border border-red-800/60 text-red-400 text-base shrink-0">
            ⚠️
          </div>
          <div class="space-y-1 min-w-0">
            <h3 class="text-sm font-semibold text-stone-100">
              {deleteConfirmModal.entry.is_dir ? "フォルダの削除" : "ファイルの削除"}
            </h3>
            <p class="text-stone-300 break-all leading-relaxed">
              「<span class="font-mono font-semibold text-stone-100">{deleteConfirmModal.entry.name}</span>」を削除してもよろしいですか？
            </p>
            {#if deleteConfirmModal.entry.is_dir}
              <p class="text-amber-400 text-[11px] pt-1 leading-relaxed">
                ※ フォルダ内のすべてのファイルおよびサブフォルダも完全に削除されます。
              </p>
            {/if}
          </div>
        </div>

        <div class="flex justify-end gap-2 pt-2 border-t border-stone-800">
          <button
            class="px-3 py-1.5 rounded-md bg-stone-800 hover:bg-stone-700 text-stone-300 hover:text-stone-100 transition cursor-pointer font-medium"
            onclick={() => (deleteConfirmModal = null)}
          >
            キャンセル
          </button>
          <button
            class="px-3 py-1.5 rounded-md bg-red-600 hover:bg-red-500 text-white font-medium transition cursor-pointer shadow-sm"
            onclick={executeDelete}
          >
            削除
          </button>
        </div>
      </div>
    </div>
  {/if}
</div>
