<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { Editor } from "@tiptap/core";
  import StarterKit from "@tiptap/starter-kit";

  // State (Svelte 5 runes)
  let editorElement = $state<HTMLDivElement | null>(null);
  let editor = $state<Editor | null>(null);
  let editMode = $state<"hybrid" | "full">("hybrid"); // "hybrid" = Block Hybrid, "full" = Full Block / WYSIWYG
  let activeFile = $state<string>("welcome.tmt");

  onMount(() => {
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
      content: `
        <h1>ようこそ Telaion へ</h1>
        <p>Telaion は <strong>Tomet</strong> 言語のためのワークスペース型ドキュメントエディタです。</p>
        <p>上部のモード切替ボタンで、ブロックハイブリッド編集とフルブロックWYSIWYG表示を切り替えられます。</p>
      `,
      editorProps: {
        attributes: {
          class: "focus:outline-none min-h-[500px] text-stone-200 text-base leading-relaxed p-6",
        },
      },
    });
  });

  onDestroy(() => {
    editor?.destroy();
  });

  function toggleMode(mode: "hybrid" | "full") {
    editMode = mode;
    // TODO: Tiptap extension やデコレーションの設定を切り替える
  }
</script>

<div class="flex h-screen w-screen overflow-hidden bg-stone-950 text-stone-200">
  <!-- Sidebar: Workspace Tree -->
  <aside class="w-64 border-r border-stone-800 bg-stone-900/60 flex flex-col shrink-0">
    <div class="p-3 border-b border-stone-800 flex items-center justify-between">
      <span class="font-semibold text-sm tracking-wide text-stone-300">TELAION</span>
      <span class="text-xs px-2 py-0.5 rounded bg-stone-800 text-stone-400">Vault</span>
    </div>
    <div class="p-2 flex-1 overflow-y-auto space-y-1 text-sm">
      <div class="px-2 py-1 text-xs font-semibold text-stone-500 uppercase tracking-wider">Workspace</div>
      <button
        class="w-full text-left px-2 py-1.5 rounded flex items-center gap-2 hover:bg-stone-800 transition {activeFile === 'welcome.tmt' ? 'bg-stone-800 text-white font-medium' : 'text-stone-400'}"
        onclick={() => activeFile = 'welcome.tmt'}
      >
        <span>📄</span>
        <span>welcome.tmt</span>
      </button>
    </div>
  </aside>

  <!-- Main Content Area -->
  <main class="flex-1 flex flex-col h-full overflow-hidden bg-stone-900/30">
    <!-- Top Bar: Mode switcher & Document info -->
    <header class="h-12 border-b border-stone-800 px-4 flex items-center justify-between shrink-0">
      <div class="flex items-center gap-3">
        <span class="text-xs font-mono text-stone-400">{activeFile}</span>
      </div>

      <!-- Mode Switcher -->
      <div class="flex items-center gap-1 bg-stone-950/80 p-0.5 rounded-lg border border-stone-800 text-xs">
        <button
          class="px-2.5 py-1 rounded transition {editMode === 'hybrid' ? 'bg-amber-600/30 text-amber-300 border border-amber-500/40 font-medium' : 'text-stone-400 hover:text-stone-200'}"
          onclick={() => toggleMode('hybrid')}
          title="ブロックUIとインライン記法編集"
        >
          ブロックハイブリッド
        </button>
        <button
          class="px-2.5 py-1 rounded transition {editMode === 'full' ? 'bg-sky-600/30 text-sky-300 border border-sky-500/40 font-medium' : 'text-stone-400 hover:text-stone-200'}"
          onclick={() => toggleMode('full')}
          title="全要素をリッチに描画"
        >
          フルブロック (WYSIWYG)
        </button>
      </div>
    </header>

    <!-- Editor Container -->
    <div class="flex-1 overflow-y-auto px-8 py-6 max-w-4xl w-full mx-auto">
      <div bind:this={editorElement} class="tiptap-editor cursor-text selection:bg-stone-700"></div>
    </div>
  </main>
</div>
