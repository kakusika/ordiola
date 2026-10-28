<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { Editor } from "@tiptap/core";
  import StarterKit from "@tiptap/starter-kit";
  import FileTreeNode from "$lib/FileTreeNode.svelte";
  import CodeEditor from "$lib/CodeEditor.svelte";
  import { Callout } from "$lib/calloutExtension";
  import type {
    FileEntry,
    VaultEntry,
    VaultState,
    InlineAction,
    ContextMenuState,
    DeleteConfirmState,
    EditorTab,
  } from "$lib/types";

  // State (Svelte 5 runes)
  let editorElement = $state<HTMLDivElement | null>(null);
  let editor = $state<Editor | null>(null);
  let isTauri = $state<boolean>(false);
  let statusMessage = $state<string>("");

  // Multi-Tab Management State
  let tabs = $state<EditorTab[]>([]);
  let activeTabId = $state<string>("");
  let activeTab = $derived(tabs.find((t) => t.id === activeTabId) || null);
  let closeConfirmTab = $state<EditorTab | null>(null);

  // Derived properties from activeTab for seamless editor integration
  let activeFile = $derived(activeTab ? activeTab.title : "");
  let activePath = $derived(activeTab ? activeTab.path : "");
  let isSaved = $derived(activeTab ? !activeTab.isModified : true);
  let editMode = $derived<"source" | "hybrid" | "full">(activeTab ? activeTab.editMode : "source");
  let rawTextContent = $derived(activeTab ? activeTab.rawContent : "");
  let isTmtFile = $derived(activeTab ? activeTab.isTmt : false);

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

  // Document metadata & configuration state
  interface TometDocMeta {
    id: string;
    title: string;
    lang: string;
    custom: Record<string, string>;
  }

  interface TometDocConfig {
    export_type: string;
    export_path: string;
  }

  let docMeta = $state<TometDocMeta>({
    id: "doc-readme",
    title: "Telaion ワークスペース",
    lang: "ja",
    custom: {},
  });

  let docConfig = $state<TometDocConfig>({
    export_type: "commonmark",
    export_path: "README.md",
  });

  let isMetaCardExpanded = $state<boolean>(false);
  let metaActiveTab = $state<"meta" | "config">("meta");
  let newCustomKey = $state<string>("");
  let newCustomValue = $state<string>("");

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
          Callout,
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

    if ((e.ctrlKey || e.metaKey) && e.key === "w") {
      e.preventDefault();
      if (activeTab) {
        requestCloseTab(activeTab);
      }
      return;
    }

    if ((e.ctrlKey || e.metaKey) && e.key === "PageDown") {
      if (tabs.length > 1) {
        e.preventDefault();
        const cur = tabs.findIndex((t) => t.id === activeTabId);
        const next = (cur + 1) % tabs.length;
        switchTab(tabs[next].id);
        return;
      }
    }

    if ((e.ctrlKey || e.metaKey) && e.key === "PageUp") {
      if (tabs.length > 1) {
        e.preventDefault();
        const cur = tabs.findIndex((t) => t.id === activeTabId);
        const prev = (cur - 1 + tabs.length) % tabs.length;
        switchTab(tabs[prev].id);
        return;
      }
    }

    if (e.key === "Escape") {
      if (closeConfirmTab) {
        closeConfirmTab = null;
        return;
      }
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
    if (!filePath) return;
    const existing = tabs.find((t) => t.path === filePath || t.id === filePath);
    if (existing) {
      await switchTab(existing.id);
      return;
    }

    const title = filePath.split("/").pop() || filePath;
    statusMessage = `読み込み中: ${title}...`;

    let text = "";
    let pmDoc: any = null;

    if (!isTauri) {
      statusMessage = `モック表示: ${title}`;
      text = filePath.endsWith(".tmt") ? defaultTometContent : `// Content of ${title}\n`;
    } else {
      try {
        const { invoke } = await import("@tauri-apps/api/core");
        text = await invoke<string>("read_text_file", { path: filePath });

        if (filePath.endsWith(".tmt")) {
          try {
            pmDoc = await invoke<any>("parse_tomet_string", { source: text });
          } catch (e: any) {
            console.warn("Tomet parse warning:", e);
          }
        }
      } catch (err: any) {
        console.error("Failed to read file:", err);
        statusMessage = `読み込みエラー: ${err?.message || err}`;
        return;
      }
    }

    if (filePath.endsWith(".tmt")) {
      await loadTometMetaAndConfig(text);
    }

    const newTab: EditorTab = {
      id: filePath,
      path: filePath,
      title,
      isModified: false,
      isTmt: filePath.endsWith(".tmt"),
      rawContent: text,
      pmDoc,
      editMode: "source",
    };

    tabs.push(newTab);
    await switchTab(newTab.id);
    statusMessage = `ロード完了: ${title}`;
  }

  async function switchTab(tabId: string) {
    if (tabId === activeTabId) return;

    if (activeTab && editor && activeTab.editMode !== "source") {
      activeTab.pmDoc = editor.getJSON();
    }

    activeTabId = tabId;

    if (activeTab && activeTab.isTmt) {
      await loadTometMetaAndConfig(activeTab.rawContent);
    }

    if (activeTab && editor && activeTab.editMode !== "source") {
      if (activeTab.pmDoc) {
        editor.commands.setContent(activeTab.pmDoc);
      } else if (activeTab.rawContent) {
        if (isTauri) {
          try {
            const { invoke } = await import("@tauri-apps/api/core");
            const parsed = await invoke<any>("parse_tomet_string", { source: activeTab.rawContent });
            editor.commands.setContent(parsed);
            activeTab.pmDoc = parsed;
          } catch (e) {
            console.warn("Tomet parse on switch warning:", e);
          }
        }
      }
    }
  }

  function requestCloseTab(tabToClose: EditorTab) {
    if (tabToClose.isModified) {
      closeConfirmTab = tabToClose;
      return;
    }
    executeCloseTab(tabToClose);
  }

  function executeCloseTab(tabToClose: EditorTab) {
    const idx = tabs.findIndex((t) => t.id === tabToClose.id);
    if (idx === -1) return;

    tabs = tabs.filter((t) => t.id !== tabToClose.id);
    if (activeTabId === tabToClose.id) {
      if (tabs.length > 0) {
        const nextIdx = Math.min(idx, tabs.length - 1);
        switchTab(tabs[nextIdx].id);
      } else {
        activeTabId = "";
      }
    }
    closeConfirmTab = null;
  }

  async function saveAndCloseTab(tab: EditorTab) {
    if (tab.id === activeTabId) {
      await saveCurrentDocument();
    } else {
      if (isTauri) {
        try {
          const { invoke } = await import("@tauri-apps/api/core");
          await invoke("write_text_file", { path: tab.path, content: tab.rawContent });
        } catch (e: any) {
          console.error("Save on close failed:", e);
        }
      }
    }
    executeCloseTab(tab);
  }

  async function loadTometMetaAndConfig(source: string) {
    if (isTauri) {
      try {
        const { invoke } = await import("@tauri-apps/api/core");
        const meta = await invoke<any>("extract_metadata", { source });
        docMeta = {
          id: meta.id || "",
          title: meta.title || "",
          lang: meta.lang || "ja",
          custom: meta.custom || {},
        };
        const cfg = await invoke<any>("extract_config", { source });
        docConfig = {
          export_type: cfg.export_type || "",
          export_path: cfg.export_path || "",
        };
      } catch (e) {
        console.warn("Failed to extract metadata/config:", e);
      }
    } else {
      const idMatch = source.match(/id:\s*([^\s,]+)/);
      const titleMatch = source.match(/title:\s*["']([^"']+)["']/);
      const langMatch = source.match(/lang:\s*([^\s,]+)/);
      docMeta = {
        id: idMatch ? idMatch[1] : "",
        title: titleMatch ? titleMatch[1] : "",
        lang: langMatch ? langMatch[1] : "ja",
        custom: {},
      };
      const expTypeMatch = source.match(/type:\s*([^\s,]+)/);
      const expPathMatch = source.match(/path:\s*["']([^"']+)["']/);
      docConfig = {
        export_type: expTypeMatch ? expTypeMatch[1] : "",
        export_path: expPathMatch ? expPathMatch[1] : "",
      };
    }
  }

  async function syncMetaToDocument() {
    if (!activeTab) return;
    if (isTauri) {
      try {
        const { invoke } = await import("@tauri-apps/api/core");
        const updated = await invoke<string>("inject_metadata", {
          source: activeTab.rawContent,
          meta: {
            id: docMeta.id || null,
            title: docMeta.title || null,
            lang: docMeta.lang || null,
            custom: docMeta.custom,
          },
        });
        activeTab.rawContent = updated;
        activeTab.isModified = true;
        statusMessage = "メタデータを文書に反映しました";
      } catch (e: any) {
        statusMessage = `メタデータ反映失敗: ${e?.message || e}`;
      }
    } else {
      statusMessage = "メタデータを更新しました（ブラウザモック）";
    }
  }

  function addCustomMetaField() {
    const k = newCustomKey.trim();
    if (k) {
      docMeta.custom[k] = newCustomValue.trim();
      docMeta.custom = { ...docMeta.custom };
      newCustomKey = "";
      newCustomValue = "";
    }
  }

  function removeCustomMetaField(key: string) {
    delete docMeta.custom[key];
    docMeta.custom = { ...docMeta.custom };
  }

  function insertTometBlock(kind: string) {
    if (!activeTab) return;
    let snippet = "";
    if (kind === "note") {
      snippet = `::{callout(note)}\nここにノート・補足事項を記述します。\n::\n`;
    } else if (kind === "tip") {
      snippet = `::{callout(tip)}\nここに有用な Tips やヒントを記述します。\n::\n`;
    } else if (kind === "warning") {
      snippet = `::{callout(warning)}\nここに注意事項・警告を記述します。\n::\n`;
    } else if (kind === "danger") {
      snippet = `::{callout(danger)}\nここに重大な危険・破壊的変更への注意を記述します。\n::\n`;
    } else if (kind === "code") {
      snippet = `::{code(typescript)}\nconsole.log("Hello, Tomet!");\n::\n`;
    }

    if (activeTab.editMode === "source") {
      activeTab.rawContent = activeTab.rawContent ? activeTab.rawContent.trimEnd() + "\n\n" + snippet : snippet;
      activeTab.isModified = true;
      statusMessage = `${kind} ブロックを挿入しました`;
    } else if (editor) {
      if (kind === "code") {
        editor.chain().focus().setCodeBlock().insertContent('console.log("Hello, Tomet!");').run();
      } else {
        editor.chain().focus().insertContent(`<div data-callout="${kind}" class="tomet-callout-card tomet-callout-${kind}"><p>ここに${kind}の本文を入力</p></div>`).run();
      }
      activeTab.isModified = true;
      statusMessage = `${kind} ブロックを挿入しました`;
    }
  }

  async function saveCurrentDocument() {
    if (!activeTab) return;
    statusMessage = "保存中...";

    if (!isTauri) {
      activeTab.isModified = false;
      statusMessage = `保存完了（ブラウザモック）: ${activeTab.title}`;
      return;
    }

    try {
      const { invoke } = await import("@tauri-apps/api/core");
      if (activeTab.editMode === "source" || !activeTab.isTmt) {
        await invoke("write_text_file", {
          path: activeTab.path,
          content: activeTab.rawContent,
        });
      } else {
        const json = editor?.getJSON();
        if (json) {
          const tometStr = await invoke<string>("serialize_prosemirror_doc", { doc: json });
          await invoke("write_text_file", {
            path: activeTab.path,
            content: tometStr,
          });
          activeTab.rawContent = tometStr;
          activeTab.pmDoc = json;
        }
      }
      activeTab.isModified = false;
      statusMessage = `保存完了: ${activeTab.path}`;
    } catch (err: any) {
      console.error("Failed to save document:", err);
      statusMessage = `保存失敗: ${err?.message || err}`;
    }
  }

  function handleSourceChange(newContent: string) {
    if (activeTab) {
      activeTab.rawContent = newContent;
      activeTab.isModified = true;
    }
  }

  async function toggleMode(mode: "source" | "hybrid" | "full") {
    if (!activeTab || mode === activeTab.editMode) return;

    if (!activeTab.isTmt && mode !== "source") {
      statusMessage = "Tomet ファイル (.tmt) 以外はソース編集モードのみ対応しています";
      return;
    }

    if (mode === "hybrid" || mode === "full") {
      if (isTauri && editor) {
        try {
          const { invoke } = await import("@tauri-apps/api/core");
          const pmDoc = await invoke<any>("parse_tomet_string", { source: activeTab.rawContent });
          if (pmDoc) {
            editor.commands.setContent(pmDoc);
            activeTab.pmDoc = pmDoc;
          }
        } catch (err: any) {
          console.warn("Could not parse Tomet into ProseMirror:", err);
          statusMessage = `構文解析エラーのためソース編集のままにします: ${err?.message || err}`;
          return;
        }
      }
    } else if (mode === "source" && (activeTab.editMode === "hybrid" || activeTab.editMode === "full")) {
      if (isTauri && editor) {
        try {
          const { invoke } = await import("@tauri-apps/api/core");
          const json = editor.getJSON();
          const tometStr = await invoke<string>("serialize_prosemirror_doc", { doc: json });
          if (tometStr) {
            activeTab.rawContent = tometStr;
            activeTab.pmDoc = json;
          }
        } catch (err: any) {
          console.warn("Could not serialize ProseMirror into Tomet:", err);
        }
      }
    }

    activeTab.editMode = mode;
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
    <!-- Top Bar: Tab Strip, Mode Switcher, Actions -->
    <header class="h-11 border-b border-stone-800/80 px-2 flex items-center justify-between shrink-0 bg-stone-900/60 backdrop-blur-sm select-none gap-2 min-w-0">
      <!-- Tabs Strip Area -->
      <div class="flex items-center gap-1 overflow-x-auto min-w-0 flex-1 scrollbar-none py-1">
        {#each tabs as tab (tab.id)}
          <div
            class="group flex items-center gap-1.5 px-3 py-1.5 rounded-md text-xs font-mono transition cursor-pointer shrink-0 border
              {tab.id === activeTabId
                ? 'bg-stone-800/90 text-stone-100 border-stone-700/80 shadow-sm'
                : 'text-stone-400 hover:text-stone-200 hover:bg-stone-800/40 border-transparent'}"
            role="tab"
            tabindex="0"
            onclick={() => switchTab(tab.id)}
            onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && switchTab(tab.id)}
            onauxclick={(e) => { if (e.button === 1) requestCloseTab(tab); }}
            title={tab.path}
          >
            <span class="text-xs shrink-0">{tab.isTmt ? "📜" : "📄"}</span>
            <span class="truncate max-w-40 {tab.id === activeTabId ? 'font-medium' : ''}">{tab.title}</span>

            {#if tab.isModified}
              <span class="w-2 h-2 rounded-full bg-amber-400 shrink-0 group-hover:hidden" title="変更あり"></span>
            {/if}

            <button
              class="w-4 h-4 rounded hover:bg-stone-700 text-stone-400 hover:text-stone-200 flex items-center justify-center text-[10px] transition shrink-0 ml-0.5 cursor-pointer
                {tab.isModified ? 'hidden group-hover:flex' : 'opacity-60 group-hover:opacity-100'}"
              onclick={(e) => { e.stopPropagation(); requestCloseTab(tab); }}
              title="タブを閉じる (Ctrl+W)"
            >
              ✕
            </button>
          </div>
        {/each}
      </div>

      <!-- Mode Switcher & Actions (Visible when a tab is open) -->
      {#if activeTab}
        <div class="flex items-center gap-2 shrink-0">
          <!-- Mode Switcher -->
          <div class="flex items-center bg-stone-950 p-0.5 rounded-lg border border-stone-800 text-xs">
            <button
              class="px-2.5 py-1 rounded transition flex items-center gap-1 cursor-pointer {editMode === 'source' ? 'bg-sky-500/20 text-sky-300 border border-sky-500/30 font-medium shadow-sm' : 'text-stone-400 hover:text-stone-200'}"
              onclick={() => toggleMode('source')}
              title="ソース編集: 生テキスト直接編集"
            >
              <span>📄</span>
              <span class="text-[11px]">ソース</span>
            </button>

            {#if isTmtFile}
              <button
                class="px-2.5 py-1 rounded transition flex items-center gap-1 cursor-pointer {editMode === 'hybrid' ? 'bg-amber-500/20 text-amber-300 border border-amber-500/30 font-medium shadow-sm' : 'text-stone-400 hover:text-stone-200'}"
                onclick={() => toggleMode('hybrid')}
                title="ブロックハイブリッド編集"
              >
                <span>⚡</span>
                <span class="text-[11px]">ハイブリッド</span>
              </button>
              <button
                class="px-2.5 py-1 rounded transition flex items-center gap-1 cursor-pointer {editMode === 'full' ? 'bg-emerald-500/20 text-emerald-300 border border-emerald-500/30 font-medium shadow-sm' : 'text-stone-400 hover:text-stone-200'}"
                onclick={() => toggleMode('full')}
                title="フルブロック (WYSIWYG) 編集"
              >
                <span>🎨</span>
                <span class="text-[11px]">フルブロック</span>
              </button>
            {/if}
          </div>

          <!-- Save Button -->
          <button
            class="px-2.5 py-1 rounded-md bg-stone-800 hover:bg-stone-700 text-stone-200 text-xs font-medium border border-stone-700/60 transition flex items-center gap-1.5 cursor-pointer shrink-0"
            onclick={saveCurrentDocument}
            title="ドキュメントを保存 (Ctrl+S)"
          >
            <span>💾</span>
            <span class="text-[11px]">保存</span>
          </button>
        </div>
      {/if}
    </header>

    {#if tabs.length === 0}
      <!-- Empty Workspace State -->
      <div class="flex-1 flex flex-col items-center justify-center text-center p-8 select-none text-stone-500">
        <div class="w-16 h-16 rounded-2xl bg-stone-900/60 border border-stone-800 flex items-center justify-center text-3xl mb-4 shadow-xl">
          📜
        </div>
        <h2 class="text-sm font-semibold text-stone-300 mb-1">開いているドキュメントがありません</h2>
        <p class="text-xs text-stone-500 max-w-sm mb-6 leading-relaxed">
          左側のエクスプローラーからファイルを選択して開くか、新しいファイルを作成してください。
        </p>
        <button
          class="px-3 py-1.5 rounded-lg bg-stone-800 hover:bg-stone-700 text-stone-200 text-xs font-medium border border-stone-700/60 transition flex items-center gap-1.5 cursor-pointer"
          onclick={() => startCreateFile(selectedDir || "")}
        >
          <span>📄</span>
          <span>新しいファイルを作成</span>
        </button>
      </div>
    {:else}

    {#if isTmtFile}
      <!-- Tomet Block Insert Bar -->
      <div class="h-9 bg-stone-900/80 border-b border-stone-800/80 px-6 flex items-center justify-between text-xs shrink-0 select-none backdrop-blur-xs">
        <div class="flex items-center gap-1.5 overflow-x-auto py-1">
          <span class="text-[10px] font-bold text-stone-500 tracking-wider uppercase mr-1">Tomet ブロック:</span>
          <button
            class="px-2 py-0.5 rounded bg-sky-950/40 hover:bg-sky-900/60 text-sky-300 hover:text-sky-200 border border-sky-500/30 flex items-center gap-1 transition cursor-pointer text-[11px]"
            onclick={() => insertTometBlock('note')}
            title="Note (補足・注記) コールアウトを挿入"
          >
            <span>💡</span>
            <span>Note</span>
          </button>
          <button
            class="px-2 py-0.5 rounded bg-emerald-950/40 hover:bg-emerald-900/60 text-emerald-300 hover:text-emerald-200 border border-emerald-500/30 flex items-center gap-1 transition cursor-pointer text-[11px]"
            onclick={() => insertTometBlock('tip')}
            title="Tip (ヒント) コールアウトを挿入"
          >
            <span>🚀</span>
            <span>Tip</span>
          </button>
          <button
            class="px-2 py-0.5 rounded bg-amber-950/40 hover:bg-amber-900/60 text-amber-300 hover:text-amber-200 border border-amber-500/30 flex items-center gap-1 transition cursor-pointer text-[11px]"
            onclick={() => insertTometBlock('warning')}
            title="Warning (注意事項) コールアウトを挿入"
          >
            <span>⚠️</span>
            <span>Warning</span>
          </button>
          <button
            class="px-2 py-0.5 rounded bg-rose-950/40 hover:bg-rose-900/60 text-rose-300 hover:text-rose-200 border border-rose-500/30 flex items-center gap-1 transition cursor-pointer text-[11px]"
            onclick={() => insertTometBlock('danger')}
            title="Danger (重大・危険) コールアウトを挿入"
          >
            <span>🛑</span>
            <span>Danger</span>
          </button>
          <button
            class="px-2 py-0.5 rounded bg-stone-800/60 hover:bg-stone-700/60 text-stone-300 hover:text-stone-100 border border-stone-700 flex items-center gap-1 transition cursor-pointer text-[11px]"
            onclick={() => insertTometBlock('code')}
            title="コードブロック (code) を挿入"
          >
            <span>💻</span>
            <span>Code Block</span>
          </button>
        </div>

        <button
          class="flex items-center gap-1.5 px-2.5 py-1 rounded bg-stone-800/40 hover:bg-stone-800 text-stone-300 hover:text-stone-100 text-[11px] border border-stone-700/50 transition cursor-pointer"
          onclick={() => (isMetaCardExpanded = !isMetaCardExpanded)}
          title="文書メタデータ・エクスポート設定パネルを開閉"
        >
          <span>📜</span>
          <span>設定: {docMeta.id || '未設定'}</span>
          <span class="text-[9px] text-stone-400">{isMetaCardExpanded ? '▲' : '▼'}</span>
        </button>
      </div>

      <!-- Collapsible Document Meta / Config Inspector Card -->
      {#if isMetaCardExpanded}
        <div class="bg-stone-900/90 border-b border-stone-800 px-6 py-3 text-xs space-y-3 backdrop-blur-md">
          <div class="flex items-center justify-between border-b border-stone-800 pb-2">
            <div class="flex items-center gap-2">
              <button
                class="px-2.5 py-1 rounded font-medium transition cursor-pointer {metaActiveTab === 'meta' ? 'bg-stone-800 text-stone-100 border border-stone-700' : 'text-stone-400 hover:text-stone-200'}"
                onclick={() => (metaActiveTab = 'meta')}
              >
                ::meta (文書情報)
              </button>
              <button
                class="px-2.5 py-1 rounded font-medium transition cursor-pointer {metaActiveTab === 'config' ? 'bg-stone-800 text-stone-100 border border-stone-700' : 'text-stone-400 hover:text-stone-200'}"
                onclick={() => (metaActiveTab = 'config')}
              >
                ::config (エクスポート設定)
              </button>
            </div>

            <button
              class="px-3 py-1 rounded bg-sky-600 hover:bg-sky-500 text-white font-medium text-[11px] shadow-sm transition cursor-pointer flex items-center gap-1"
              onclick={syncMetaToDocument}
              title="変更したメタデータをドキュメント先頭に反映します"
            >
              <span>🔄</span>
              <span>文書に反映</span>
            </button>
          </div>

          {#if metaActiveTab === 'meta'}
            <div class="grid grid-cols-3 gap-3">
              <div>
                <span class="text-stone-400 block text-[10px] mb-1 font-mono">id:</span>
                <input
                  type="text"
                  bind:value={docMeta.id}
                  class="w-full bg-stone-950 border border-stone-700 rounded px-2.5 py-1 text-stone-100 font-mono text-xs focus:outline-none focus:border-sky-500"
                  placeholder="doc-id"
                />
              </div>
              <div>
                <span class="text-stone-400 block text-[10px] mb-1 font-mono">title:</span>
                <input
                  type="text"
                  bind:value={docMeta.title}
                  class="w-full bg-stone-950 border border-stone-700 rounded px-2.5 py-1 text-stone-100 text-xs focus:outline-none focus:border-sky-500"
                  placeholder="ドキュメントのタイトル"
                />
              </div>
              <div>
                <span class="text-stone-400 block text-[10px] mb-1 font-mono">lang:</span>
                <input
                  type="text"
                  bind:value={docMeta.lang}
                  class="w-full bg-stone-950 border border-stone-700 rounded px-2.5 py-1 text-stone-100 font-mono text-xs focus:outline-none focus:border-sky-500"
                  placeholder="ja / en"
                />
              </div>
            </div>

            <!-- Custom properties -->
            <div class="pt-2 border-t border-stone-800/80 space-y-1.5">
              <span class="text-[10px] font-semibold text-stone-400 uppercase tracking-wider block">カスタム属性:</span>
              <div class="space-y-1">
                {#each Object.entries(docMeta.custom) as [k, v] (k)}
                  <div class="flex items-center gap-2">
                    <span class="font-mono text-stone-300 w-28 truncate text-[11px]">{k}:</span>
                    <input
                      type="text"
                      bind:value={docMeta.custom[k]}
                      class="flex-1 bg-stone-950 border border-stone-800 rounded px-2 py-0.5 text-stone-200 text-xs focus:outline-none focus:border-stone-600"
                    />
                    <button
                      class="px-2 py-0.5 text-stone-500 hover:text-red-400 transition cursor-pointer text-xs"
                      onclick={() => removeCustomMetaField(k)}
                      title="属性を削除"
                    >
                      ✕
                    </button>
                  </div>
                {/each}
              </div>

              <!-- Add custom attribute row -->
              <div class="flex items-center gap-2 pt-1">
                <input
                  type="text"
                  bind:value={newCustomKey}
                  placeholder="新しいキー (例: author)"
                  class="w-36 bg-stone-950 border border-stone-800 rounded px-2 py-0.5 text-stone-300 font-mono text-[11px] focus:outline-none focus:border-sky-500"
                />
                <input
                  type="text"
                  bind:value={newCustomValue}
                  placeholder="値"
                  class="flex-1 bg-stone-950 border border-stone-800 rounded px-2 py-0.5 text-stone-300 text-[11px] focus:outline-none focus:border-sky-500"
                  onkeydown={(e) => { if (e.key === 'Enter') addCustomMetaField(); }}
                />
                <button
                  class="px-2.5 py-0.5 rounded bg-stone-800 hover:bg-stone-700 text-stone-200 text-[11px] transition cursor-pointer"
                  onclick={addCustomMetaField}
                >
                  ＋ 追加
                </button>
              </div>
            </div>
          {:else}
            <!-- Config Tab -->
            <div class="grid grid-cols-2 gap-4">
              <div>
                <span class="text-stone-400 block text-[10px] mb-1 font-mono">export.type:</span>
                <input
                  type="text"
                  bind:value={docConfig.export_type}
                  class="w-full bg-stone-950 border border-stone-700 rounded px-2.5 py-1 text-stone-100 font-mono text-xs focus:outline-none focus:border-sky-500"
                  placeholder="commonmark"
                />
              </div>
              <div>
                <span class="text-stone-400 block text-[10px] mb-1 font-mono">export.path:</span>
                <input
                  type="text"
                  bind:value={docConfig.export_path}
                  class="w-full bg-stone-950 border border-stone-700 rounded px-2.5 py-1 text-stone-100 font-mono text-xs focus:outline-none focus:border-sky-500"
                  placeholder="README.md"
                />
              </div>
            </div>
          {/if}
        </div>
      {/if}
    {/if}

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
    {/if}
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
  <!-- Unsaved Changes on Tab Close Modal -->
  {#if closeConfirmTab}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="fixed inset-0 z-50 bg-black/60 backdrop-blur-xs flex items-center justify-center p-4"
      onclick={() => (closeConfirmTab = null)}
    >
      <div
        class="bg-stone-900 border border-stone-700 rounded-xl shadow-2xl p-5 max-w-sm w-full text-xs space-y-3"
        onclick={(e) => e.stopPropagation()}
      >
        <div class="flex items-start gap-3">
          <div class="p-2 rounded-lg bg-amber-950/60 border border-amber-800/60 text-amber-400 text-base shrink-0">
            ⚠️
          </div>
          <div class="space-y-1 min-w-0">
            <h3 class="text-sm font-semibold text-stone-100">変更が保存されていません</h3>
            <p class="text-stone-300 break-all leading-relaxed">
              「<span class="font-mono font-semibold text-stone-100">{closeConfirmTab.title}</span>」への変更を保存しますか？
            </p>
          </div>
        </div>

        <div class="flex justify-end gap-2 pt-2 border-t border-stone-800">
          <button
            class="px-3 py-1.5 rounded-md bg-stone-800 hover:bg-stone-700 text-stone-300 hover:text-stone-100 transition cursor-pointer font-medium"
            onclick={() => (closeConfirmTab = null)}
          >
            キャンセル
          </button>
          <button
            class="px-3 py-1.5 rounded-md bg-stone-800 hover:bg-stone-700 text-amber-300 hover:text-amber-200 transition cursor-pointer font-medium"
            onclick={() => executeCloseTab(closeConfirmTab!)}
          >
            保存しない
          </button>
          <button
            class="px-3 py-1.5 rounded-md bg-sky-600 hover:bg-sky-500 text-white font-medium transition cursor-pointer shadow-sm"
            onclick={() => saveAndCloseTab(closeConfirmTab!)}
          >
            保存する
          </button>
        </div>
      </div>
    </div>
  {/if}
</div>
