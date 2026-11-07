<script lang="ts">
  // The inspector of a Tomet document: its `::meta` (information) and `::config` (export) as fields.
  import { Button, Field, IconButton, Input, Tabs } from "@telaion/primitives";
  import { Plus, RefreshCw, X } from "@telaion/primitives/icons";
  import type { TometDocConfig, TometDocMeta } from "$lib/types";

  let {
    meta = $bindable(),
    config = $bindable(),
    id,
    onapply,
  }: {
    meta: TometDocMeta;
    config: TometDocConfig;
    /** The element id of the inspector, for the switch that opens it to point at. */
    id: string;
    /** Called when the person asks for the changes to be written into the document. */
    onapply: () => void;
  } = $props();

  const tabs = [
    { id: "meta", label: "::meta (文書情報)" },
    { id: "config", label: "::config (エクスポート設定)" },
  ];

  let tab = $state("meta");
  let newKey = $state("");
  let newValue = $state("");

  function addCustom() {
    const key = newKey.trim();
    if (!key) return;
    meta.custom = { ...meta.custom, [key]: newValue.trim() };
    newKey = "";
    newValue = "";
  }

  function removeCustom(key: string) {
    const { [key]: _removed, ...rest } = meta.custom;
    meta.custom = rest;
  }
</script>

<!-- A surface that only holds controls is content; the fields on it are wells. -->
<section class="inspector" {id} aria-label="文書メタデータ">
  <header class="head">
    <Tabs items={tabs} bind:value={tab} size="sm" label="メタデータの種類" />
    <Button variant="primary" size="sm" icon={RefreshCw} title="変更したメタデータをドキュメント先頭に反映します" onclick={onapply}>文書に反映</Button>
  </header>

  {#if tab === "meta"}
    <div class="columns three">
      <Field label="id"><Input size="sm" mono bind:value={meta.id} placeholder="doc-id" /></Field>
      <Field label="title"><Input size="sm" bind:value={meta.title} placeholder="ドキュメントのタイトル" /></Field>
      <Field label="lang"><Input size="sm" mono bind:value={meta.lang} placeholder="ja / en" /></Field>
    </div>

    <div class="custom" role="group" aria-labelledby="{id}-custom">
      <h3 class="heading" id="{id}-custom">カスタム属性</h3>
      {#each Object.keys(meta.custom) as key (key)}
        <div class="row">
          <Field label={key}><Input size="sm" bind:value={meta.custom[key]} /></Field>
          <IconButton size="sm" icon={X} label="属性「{key}」を削除" onclick={() => removeCustom(key)} />
        </div>
      {/each}
      <div class="row">
        <Field label="新しいキー">
          <Input size="sm" mono bind:value={newKey} placeholder="新しいキー (例: author)" />
        </Field>
        <Field label="値">
          <Input
            size="sm"
            bind:value={newValue}
            placeholder="値"
            onkeydown={(event) => {
              if (event.key === "Enter") addCustom();
            }}
          />
        </Field>
        <Button size="sm" icon={Plus} disabled={!newKey.trim()} onclick={addCustom}>追加</Button>
      </div>
    </div>
  {:else}
    <div class="columns two">
      <Field label="export.type"><Input size="sm" mono bind:value={config.export_type} placeholder="commonmark" /></Field>
      <Field label="export.path"><Input size="sm" mono bind:value={config.export_path} placeholder="README.md" /></Field>
    </div>
  {/if}
</section>

<style>
  .inspector {
    display: grid;
    flex: none;
    gap: calc(var(--tln-space) * 3);
    padding: calc(var(--tln-space) * 3);
    color: var(--tln-fg);
    background: var(--tln-content);
  }

  .head {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: calc(var(--tln-space) * 2);
  }

  .columns {
    display: grid;
    gap: calc(var(--tln-space) * 3);
  }

  .three {
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }

  .two {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .custom {
    display: grid;
    gap: calc(var(--tln-space) * 2);
  }

  .heading {
    margin: 0;
    font-size: var(--tln-text-xs);
    font-weight: var(--tln-weight-medium);
    color: var(--tln-fg-muted);
  }

  /* A field with the control that acts on it beside it, level with the input rather than its label. */
  .row {
    display: flex;
    align-items: flex-end;
    gap: calc(var(--tln-space) * 2);
  }

  .row > :global(.tln-field) {
    flex: 1 1 0;
    min-width: 0;
  }
</style>
