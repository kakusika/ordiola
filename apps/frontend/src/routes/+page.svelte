<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import type { Editor } from "@tiptap/core";
  import ExplorerPane from "$lib/ExplorerPane.svelte";
  import VaultSwitcher from "$lib/VaultSwitcher.svelte";
  import { creationDir, entryPath, joinPath, mockDetach, mockInsert, mockRelocate, parentPath, remapPath } from "$lib/explorer";
  import type { NewEntryKind } from "$lib/explorer";
  import { CodeEditor, sourceEditingExtensions } from "@telaion/editor";
  import { RichText } from "@telaion/editor/rich-text";
  import { Callout, HeadingMarkers } from "@telaion/tomet/rich-text";
  import { SplitPane } from "@telaion/shell";
  import { Kanban } from "@telaion/views";
  import type { KanbanColumnData } from "@telaion/views";
  import { sourceLanguage } from "$lib/sourceEditor";
  import { ConfirmDialog } from "@telaion/primitives";
  import ConfirmDelete from "$lib/ConfirmDelete.svelte";
  import QuickOpen from "$lib/QuickOpen.svelte";
  import EmptyWorkspace from "$lib/EmptyWorkspace.svelte";
  import ModeBanner from "$lib/ModeBanner.svelte";
  import StatusBar from "$lib/StatusBar.svelte";
  import WorkspaceHeader from "$lib/WorkspaceHeader.svelte";
  import MetaInspector from "$lib/MetaInspector.svelte";
  import TometBlockBar from "$lib/TometBlockBar.svelte";
  import { mockProseMirrorDoc } from "$lib/mockDocument";
  import { TypstSplitView } from "@ordiola/ext-typst";
  import type {
    FileEntry,
    VaultEntry,
    VaultState,
    DeleteConfirmState,
    EditorTab,
    TometDocConfig,
    TometDocMeta,
  } from "$lib/types";

  // State (Svelte 5 runes)
  // The Tiptap editor of the block modes, bound from `RichText`; there is one only while one of them is shown.
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
  let editMode = $derived<"source" | "hybrid" | "full" | "kanban">(activeTab ? activeTab.editMode : "source");
  let rawTextContent = $derived(activeTab ? activeTab.rawContent : "");
  let isTmtFile = $derived(activeTab ? activeTab.isTmt : false);
  let isTypstFile = $derived(activeTab ? activeTab.isTypst : false);
  // The block modes edit the ProseMirror document of the tab; the other modes edit its text.
  let isBlockMode = $derived(editMode === "hybrid" || editMode === "full");

  // Vault State
  let vaultName = $state<string>("ordiola");
  let vaultPath = $state<string>(".");
  let recentVaults = $state<VaultEntry[]>([]);

  // File Tree Explorer State. `expandedPaths` is replaced, never changed in place: the tree reads it.
  let rootEntries = $state<FileEntry[]>([]);
  let expandedPaths = $state<Set<string>>(new Set());
  let selectedTreeId = $state<string | null>(null);
  let isRefreshing = $state<boolean>(false);
  let explorer = $state<ReturnType<typeof ExplorerPane>>();
  let unlistenWatcher: (() => void) | null = null;

  $effect(() => {
    if (activePath) {
      selectedTreeId = activePath;
    }
  });

  // Hybrid draws the Tomet markers around headings, full does not: a state of the editor, not of the document.
  $effect(() => {
    editor?.commands.setHeadingMarkers(editMode === "hybrid");
  });

  let kanbanColumns = $state<KanbanColumnData[]>([
    {
      id: "todo",
      title: "ToDo",
      items: [
        { id: "task-1", title: "設計仕様の策定", description: "Telaion トークンモデルの適用", badge: "仕様", badgeTone: "neutral" },
        { id: "task-2", title: "コンポーネント分離", description: "primitives, shell, views の分割", badge: "設計", badgeTone: "accent" },
      ],
    },
    {
      id: "inprogress",
      title: "進行中",
      items: [
        { id: "task-3", title: "Ordiola 統合テスト", description: "SplitPane と TreeExplorer の導入検証", badge: "検証", badgeTone: "warn" },
      ],
    },
    {
      id: "done",
      title: "完了",
      items: [
        { id: "task-4", title: "@telaion/theme 0.1.5 公開", description: "npm レジストリへ 0.1.5 をパブリッシュ", badge: "リリース", badgeTone: "ok" },
      ],
    },
  ]);

  // Where a new file goes when nothing says otherwise: the selected directory, else beside the selected file.
  let selectedDir = $derived(creationDir(rootEntries, selectedTreeId));

  let deleteConfirmModal = $state<DeleteConfirmState | null>(null);

  // Quick Open: the workspace files are loaded when it opens; QuickOpen does the matching.
  let isQuickOpenOpen = $state<boolean>(false);
  let allWorkspaceFiles = $state<FileEntry[]>([]);

  // Document metadata & configuration state
  let docMeta = $state<TometDocMeta>({
    id: "doc-readme",
    title: "Ordiola ワークスペース",
    lang: "ja",
    custom: {},
  });

  let docConfig = $state<TometDocConfig>({
    export_type: "commonmark",
    export_path: "README.md",
  });

  let isMetaCardExpanded = $state<boolean>(false);

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

=[ Ordiola: 次世代 Tomet エディタ ]=

Ordiola は Tomet 言語のためのワークスペース型デスクトップエディタです。

==[ 主な特徴 ]==

- Vault 管理: 複数のワークスペースを瞬時に切り替え
- ソース編集: CodeMirror 6 による安全・快適な生テキスト編集
- ブロックハイブリッド: ブロック要素とマークアップのハイブリッド体験
- フルブロック (WYSIWYG): 装飾記号を隠したリッチテキスト執筆
`;

  onMount(async () => {
    isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

    // Load initial vault and workspace files via Tauri IPC
    if (isTauri) {
      await initVault();
      setupFileWatcher();
    } else {
      rawTextContent = defaultTometContent;
      // Mock vaults and entries for browser dev
      recentVaults = [
        { name: "ordiola", path: "/home/user/projects/ordiola", last_opened: 2 },
        { name: "notes", path: "/home/user/documents/notes/with/a/rather/long/path/to/show/how/it/shortens", last_opened: 1 },
      ];
      vaultPath = recentVaults[0].path;
      expandedPaths = new Set(["tmtroot"]);
      rootEntries = [
        {
          name: "tmtroot",
          path: "tmtroot",
          relative_path: "tmtroot",
          is_dir: true,
          has_children: true,
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
  });

  function handleKeydown(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && (e.key === "p" || e.key === "P")) {
      e.preventDefault();
      toggleQuickOpen();
      return;
    }

    if ((e.ctrlKey || e.metaKey) && (e.key === "s" || e.key === "S")) {
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
  }

  async function openQuickOpen() {
    isQuickOpenOpen = true;
    if (isTauri) {
      try {
        const { invoke } = await import("@tauri-apps/api/core");
        const files = await invoke<FileEntry[]>("list_all_files");
        allWorkspaceFiles = files || [];
      } catch (e) {
        console.error("Failed to list all files for quick open:", e);
      }
    } else {
      allWorkspaceFiles = [
        { name: "readme.tmt", path: "tmtroot/readme.tmt", relative_path: "tmtroot/readme.tmt", is_dir: false, has_children: false, extension: "tmt" },
        { name: "agents.tmt", path: "tmtroot/agents.tmt", relative_path: "tmtroot/agents.tmt", is_dir: false, has_children: false, extension: "tmt" },
        { name: "contributing.tmt", path: "tmtroot/contributing.tmt", relative_path: "tmtroot/contributing.tmt", is_dir: false, has_children: false, extension: "tmt" },
        { name: "Cargo.toml", path: "Cargo.toml", relative_path: "Cargo.toml", is_dir: false, has_children: false, extension: "toml" },
        { name: "default.config.tmt", path: "default.config.tmt", relative_path: "default.config.tmt", is_dir: false, has_children: false, extension: "tmt" },
      ];
    }
  }

  function toggleQuickOpen() {
    if (isQuickOpenOpen) {
      isQuickOpenOpen = false;
    } else {
      openQuickOpen();
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
    if (!targetPath.trim()) return;
    if (!isTauri) {
      statusMessage = "ブラウザ環境では Vault を切り替えられません";
      return;
    }
    statusMessage = `Vault 切り替え中: ${targetPath}...`;

    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const res = await invoke<{ state: VaultState; files: FileEntry[] }>("switch_vault", {
        path: targetPath,
      });

      vaultName = res.state.current_name;
      vaultPath = res.state.current_path;
      recentVaults = res.state.recent_vaults;
      expandedPaths = new Set();
      rootEntries = res.files || [];

      statusMessage = `Vault 開封: ${vaultName}`;

      // Automatically auto-expand tmtroot if present and open first file
      await autoOpenInitialDocument(rootEntries);
    } catch (err: any) {
      console.error("Switch vault error:", err);
      statusMessage = `Vault 切り替えエラー: ${err?.message || err}`;
    }
  }

  async function openFolderDialog() {
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
      await expandDirectory(tmtroot);
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

  // A refresh asked for while one is running (a file was created as the last one was reading) is not lost:
  // what the running one read may already be out of date, so it runs once more when that one is done.
  let refreshAgain = false;

  async function refreshWorkspace() {
    if (!isTauri) return;
    if (isRefreshing) {
      refreshAgain = true;
      return;
    }
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
    if (refreshAgain) {
      refreshAgain = false;
      await refreshWorkspace();
    }
  }

  async function rehydrateExpanded(entries: FileEntry[], openPaths: Set<string>) {
    const { invoke } = await import("@tauri-apps/api/core");
    for (const entry of entries) {
      if (entry.is_dir && openPaths.has(entry.relative_path)) {
        try {
          const children = await invoke<FileEntry[]>("list_directory", { path: entry.relative_path });
          entry.children = children;
          if (children.length > 0) {
            await rehydrateExpanded(children, openPaths);
          }
        } catch (e) {
          console.error("Rehydrate error on", entry.relative_path, e);
        }
      }
    }
  }

  /** Lists the children of a directory. Done each time it is opened, so that what is shown is what is on disk. */
  async function loadChildren(entry: FileEntry) {
    if (!entry.is_dir || !isTauri) return;
    entry.isLoading = true;
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      entry.children = (await invoke<FileEntry[]>("list_directory", { path: entry.relative_path })) || [];
    } catch (err: any) {
      console.error("Failed to list directory:", err);
      statusMessage = `ディレクトリ展開エラー: ${err?.message || err}`;
    } finally {
      entry.isLoading = false;
    }
  }

  function expand(...paths: string[]) {
    expandedPaths = new Set([...expandedPaths, ...paths]);
  }

  async function expandDirectory(entry: FileEntry) {
    await loadChildren(entry);
    expand(entryPath(entry));
  }

  // The explorer is where a name is typed; these start it. A directory of "" is the root of the vault.
  function startCreateFile(parentDir?: string) {
    void explorer?.startCreate("file", parentDir);
  }

  function startCreateFolder(parentDir?: string) {
    void explorer?.startCreate("folder", parentDir);
  }

  function confirmDelete(entry: FileEntry) {
    deleteConfirmModal = {
      visible: true,
      entry,
    };
  }

  /** A file or directory (and what is in it) went from path `from` to path `to`: what shows it follows. */
  function followPath(from: string, to: string) {
    for (const tab of tabs) {
      const path = remapPath(tab.path, from, to);
      if (path === tab.path) continue;
      if (tab.id === activeTabId) activeTabId = path;
      tab.id = tab.path = path;
      tab.title = path.substring(path.lastIndexOf("/") + 1);
    }
    expandedPaths = new Set([...expandedPaths].map((path) => remapPath(path, from, to)));
    if (selectedTreeId) selectedTreeId = remapPath(selectedTreeId, from, to);
  }

  /** A name was typed for a new file or folder. A message in return refuses it. */
  async function createEntry(directory: string, name: string, kind: NewEntryKind): Promise<string | void> {
    const target = joinPath(directory, name);

    if (!isTauri) {
      mockInsert(rootEntries, directory, {
        name,
        path: target,
        relative_path: target,
        is_dir: kind === "folder",
        has_children: false,
        children: kind === "folder" ? [] : undefined,
      });
      if (kind === "file") void openDocument(target);
      return;
    }

    try {
      const { invoke } = await import("@tauri-apps/api/core");
      if (kind === "file") {
        const initialContent = target.endsWith(".tmt")
          ? `=[ ${name.replace(/\.tmt$/, "")} ]=\n\n`
          : target.endsWith(".typ")
          ? `= ${name.replace(/\.typ$/, "")}\n\nWrite your Typst document here.\n`
          : "";
        await invoke<FileEntry>("create_file", { path: target, content: initialContent });
      } else {
        await invoke<FileEntry>("create_directory", { path: target });
        expand(target);
      }
      if (directory) expand(directory);
      await refreshWorkspace();
      if (kind === "file") {
        void openDocument(target).then(() => (statusMessage = `ファイル作成: ${target}`));
      } else {
        statusMessage = `フォルダ作成: ${target}`;
      }
    } catch (err: any) {
      console.error("File operation error:", err);
      return (statusMessage = `操作失敗: ${err?.message || err}`);
    }
  }

  /** A new name was typed for an entry. A message in return refuses it. */
  async function renameEntry(entry: FileEntry, name: string): Promise<string | void> {
    const from = entryPath(entry);
    const to = joinPath(parentPath(from), name);

    if (!isTauri) {
      mockRelocate(entry, from, to);
      followPath(from, to);
      return;
    }

    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke<FileEntry>("rename_entry", { oldPath: from, newName: name });
      followPath(from, to);
      await refreshWorkspace();
      statusMessage = `名前変更: ${name}`;
    } catch (err: any) {
      console.error("File operation error:", err);
      return (statusMessage = `操作失敗: ${err?.message || err}`);
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

  /** An entry was dropped into a directory (`""` is the root of the vault). */
  async function moveEntry(entry: FileEntry, directory: string) {
    const from = entryPath(entry);
    if (parentPath(from) === directory) return;

    if (!isTauri) {
      const moved = mockDetach(rootEntries, from);
      if (!moved) return;
      const to = joinPath(directory, moved.name);
      mockRelocate(moved, from, to);
      mockInsert(rootEntries, directory, moved);
      followPath(from, to);
      statusMessage = `移動完了（モック）: ${from} -> ${to}`;
      return;
    }

    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const movedEntry = await invoke<FileEntry>("move_entry", {
        sourcePath: from,
        destDir: directory || ".",
      });
      followPath(from, movedEntry.relative_path);
      await refreshWorkspace();
      statusMessage = `移動完了: ${from} -> ${movedEntry.relative_path}`;
    } catch (err: any) {
      console.error("Move error:", err);
      statusMessage = `移動失敗: ${err?.message || err}`;
    }
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
      isTypst: filePath.endsWith(".typ"),
      rawContent: text,
      pmDoc,
      editMode: "source",
    };

    tabs.push(newTab);
    await switchTab(newTab.id);
    statusMessage = `ロード完了: ${title}`;
  }

  // The engine turns Tomet text into ProseMirror JSON and back. The browser build has none, so it answers
  // with one fixed document (see mockDocument.ts) and cannot write a document out.
  async function parseTomet(source: string): Promise<any> {
    if (!isTauri) return mockProseMirrorDoc;
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<any>("parse_tomet_string", { source });
  }

  async function serializeTomet(doc: unknown): Promise<string> {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<string>("serialize_prosemirror_doc", { doc: $state.snapshot(doc) });
  }

  /** The text of a tab: written out from its ProseMirror document while it is in a block mode, else its own. */
  async function textOf(tab: EditorTab): Promise<string> {
    const inBlockMode = tab.editMode === "hybrid" || tab.editMode === "full";
    if (!isTauri || !tab.isTmt || !inBlockMode || !tab.pmDoc) return tab.rawContent;
    return serializeTomet(tab.pmDoc);
  }

  async function switchTab(tabId: string) {
    if (tabId === activeTabId) return;

    activeTabId = tabId;

    if (activeTab && activeTab.isTmt) {
      await loadTometMetaAndConfig(activeTab.rawContent);
    }

    // The document of a tab in a block mode is `pmDoc`, which RichText shows; one not parsed yet is parsed now.
    if (activeTab && isBlockMode && !activeTab.pmDoc && activeTab.rawContent) {
      try {
        activeTab.pmDoc = await parseTomet(activeTab.rawContent);
      } catch (e) {
        console.warn("Tomet parse on switch warning:", e);
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
          await invoke("write_text_file", { path: tab.path, content: await textOf(tab) });
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
      // The same button again takes the block away; a callout of another kind changes the kind.
      const chain = editor.chain().focus();
      (kind === "code" ? chain.toggleCodeBlock() : chain.toggleCallout({ type: kind })).run();
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

    const tab = activeTab;
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const content = await textOf(tab);
      await invoke("write_text_file", { path: tab.path, content });
      tab.rawContent = content;
      tab.isModified = false;
      statusMessage = `保存完了: ${tab.path}`;
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

  async function toggleMode(mode: "source" | "hybrid" | "full" | "kanban") {
    if (!activeTab || mode === activeTab.editMode) return;

    if (!activeTab.isTmt && mode !== "source" && mode !== "kanban") {
      statusMessage = "Tomet ファイル (.tmt) 以外はソースまたはカンバン編集モードのみ対応しています";
      return;
    }

    // The text and the ProseMirror document are two forms of one document, and only one of them is edited at
    // a time. Going into the block modes makes the document from the text; leaving them writes the text from
    // the document. Between hybrid and full the editor is the same one and nothing is converted.
    const tab = activeTab;
    const toBlocks = (mode === "hybrid" || mode === "full") && !isBlockMode;
    const fromBlocks = (mode === "source" || mode === "kanban") && isBlockMode;

    if (toBlocks) {
      try {
        const pmDoc = await parseTomet(tab.rawContent);
        if (pmDoc) tab.pmDoc = pmDoc;
      } catch (err: any) {
        console.warn("Could not parse Tomet into ProseMirror:", err);
        statusMessage = `構文解析エラーのためソース編集のままにします: ${err?.message || err}`;
        return;
      }
    } else if (fromBlocks) {
      try {
        tab.rawContent = await textOf(tab);
      } catch (err: any) {
        console.warn("Could not serialize ProseMirror into Tomet:", err);
        statusMessage = `Tomet への書き出しに失敗したためブロック編集のままにします: ${err?.message || err}`;
        return;
      }
    }

    tab.editMode = mode;
  }
</script>

<div class="flex h-screen w-screen overflow-hidden bg-tln-canvas text-tln-fg select-none">
  <SplitPane direction="horizontal" ratio={0.22} min={0.15} max={0.45}>
    {#snippet primary()}
      <!-- Left Sidebar: Workspace Vault & Lazy Tree Explorer -->
      <aside class="h-full w-full border-r border-tln-border bg-tln-content flex flex-col relative overflow-hidden">
    <VaultSwitcher name={vaultName} path={vaultPath} recent={recentVaults} onswitch={switchVault} onpick={openFolderDialog} />

    <ExplorerPane
      bind:this={explorer}
      entries={rootEntries}
      bind:selectedId={selectedTreeId}
      bind:expandedIds={expandedPaths}
      refreshing={isRefreshing}
      onload={loadChildren}
      onopen={(file) => openDocument(file.path || file.relative_path)}
      oncreate={createEntry}
      onrename={renameEntry}
      onmove={moveEntry}
      ondelete={confirmDelete}
      onrefresh={refreshWorkspace}
    />

    <StatusBar backend={isTauri ? "Tauri v3 IPC" : "Web (Vite)"} message={statusMessage} />
  </aside>
  {/snippet}
  {#snippet secondary()}
  <!-- Main Workspace Canvas -->
  <main class="h-full w-full flex flex-col overflow-hidden bg-tln-canvas">
    <WorkspaceHeader
      {tabs}
      activeId={activeTabId}
      onselect={switchTab}
      onclose={requestCloseTab}
      onquickopen={openQuickOpen}
      onmode={toggleMode}
      onsave={saveCurrentDocument}
    />

    {#if tabs.length === 0}
      <EmptyWorkspace oncreate={() => startCreateFile(selectedDir || "")} />
    {:else}

    {#if isTmtFile && editMode !== "kanban"}
      <TometBlockBar metaId={docMeta.id} bind:metaOpen={isMetaCardExpanded} controls="meta-inspector" oninsert={insertTometBlock} />

      {#if isMetaCardExpanded}
        <MetaInspector id="meta-inspector" bind:meta={docMeta} bind:config={docConfig} onapply={syncMetaToDocument} />
      {/if}
    {/if}

    {#if isTypstFile}
      <!-- Typst 2-Pane Editor + Live SVG Preview -->
      <div class="flex-1 overflow-hidden relative">
        <TypstSplitView
          content={rawTextContent}
          path={activePath}
          language={sourceLanguage(activePath)}
          extensions={sourceEditingExtensions}
          onChange={handleSourceChange}
        />
      </div>
    {:else}
      <!-- CodeMirror 6 Source Text Editor (Active when editMode === 'source') -->
      <div class="flex-1 overflow-hidden relative" class:hidden={editMode !== 'source'}>
      <div class="h-full w-full overflow-hidden bg-tln-content select-text">
        <CodeEditor
          variant="bare"
          label="ソース"
          content={rawTextContent}
          language={sourceLanguage(activePath)}
          extensions={sourceEditingExtensions}
          onChange={handleSourceChange}
        />
      </div>
    </div>

    <!-- Block editor (Active when editMode === 'hybrid' || editMode === 'full'): the document of the tab is its pmDoc -->
    {#if isBlockMode && activeTab}
      <div class="flex-1 min-h-0 flex flex-col bg-tln-content select-text">
        <div class="shrink-0 p-3">
          <ModeBanner mode={editMode === "hybrid" ? "hybrid" : "full"} />
        </div>
        <div class="flex-1 min-h-0">
          <!-- One editor for each tab, so that undo cannot reach into the document of another tab. -->
          {#key activeTabId}
            <RichText
              bind:content={() => activeTab?.pmDoc ?? null, (doc) => activeTab && (activeTab.pmDoc = doc)}
              bind:editor
              variant="bare"
              label="ブロックエディタ"
              extensions={[Callout, HeadingMarkers]}
              onChange={() => activeTab && (activeTab.isModified = true)}
            />
          {/key}
        </div>
      </div>
    {/if}

    <!-- Kanban Board View (Active when editMode === 'kanban') -->
    {#if editMode === 'kanban'}
      <div class="flex-1 overflow-auto p-6 bg-tln-canvas">
        <Kanban
          bind:columns={kanbanColumns}
          onaddcard={(colId) => {
            const col = kanbanColumns.find((c) => c.id === colId);
            if (col) {
              const num = col.items.length + 1;
              col.items.push({
                id: `card-${Date.now()}`,
                title: `新規タスク ${num}`,
                description: `${activeFile} のタスク項目`,
                badge: col.title,
                badgeTone: colId === "done" ? "ok" : colId === "inprogress" ? "accent" : "neutral",
              });
              kanbanColumns = [...kanbanColumns];
            }
          }}
        />
      </div>
    {/if}
    {/if}
    {/if}
  </main>
  {/snippet}
  </SplitPane>

  {#if deleteConfirmModal && deleteConfirmModal.visible}
    <ConfirmDelete
      entry={deleteConfirmModal.entry}
      onconfirm={executeDelete}
      oncancel={() => (deleteConfirmModal = null)}
    />
  {/if}

  {#if closeConfirmTab}
    {@const tab = closeConfirmTab}
    {#snippet unsavedMessage()}
      「<code class="font-tln-mono">{tab.title}</code>」への変更を保存しますか？
    {/snippet}
    <ConfirmDialog
      open
      tone="warn"
      title="変更が保存されていません"
      message={unsavedMessage}
      confirm="保存する"
      alternative="保存しない"
      alternativeVariant="danger"
      cancel="キャンセル"
      onconfirm={() => saveAndCloseTab(tab)}
      onalternative={() => executeCloseTab(tab)}
      oncancel={() => (closeConfirmTab = null)}
    />
  {/if}

  <QuickOpen bind:open={isQuickOpenOpen} files={allWorkspaceFiles} onopen={openDocument} />
</div>
