// What the browser build answers to `parse_tomet_string`, where there is no Tauri backend and so no
// Tomet engine: one fixed document in the shape of the ProseMirror JSON the engine returns (headings,
// paragraphs, lists, a code block, and `callout` nodes with a `type`), so that the block modes can be
// looked at and tried without the desktop app. The text is not derived from the source.

const text = (value: string, marks?: { type: string }[]) => ({ type: "text", text: value, ...(marks ? { marks } : {}) });
const paragraph = (...content: object[]) => ({ type: "paragraph", content });
const heading = (level: number, value: string) => ({ type: "heading", attrs: { level }, content: [text(value)] });
const item = (value: string) => ({ type: "listItem", content: [paragraph(text(value))] });
const callout = (type: string, value: string) => ({ type: "callout", attrs: { type }, content: [paragraph(text(value))] });

export const mockProseMirrorDoc = {
  type: "doc",
  content: [
    heading(1, "Ordiola: 次世代 Tomet エディタ"),
    paragraph(
      text("Ordiola は "),
      text("Tomet", [{ type: "bold" }]),
      text(" 言語のためのワークスペース型デスクトップエディタです。"),
    ),
    heading(2, "主な特徴"),
    { type: "bulletList", content: [
      item("Vault 管理: 複数のワークスペースを瞬時に切り替え"),
      item("ソース編集: CodeMirror 6 による安全・快適な生テキスト編集"),
      item("ブロックハイブリッド: ブロック要素とマークアップのハイブリッド体験"),
      item("フルブロック (WYSIWYG): 装飾記号を隠したリッチテキスト執筆"),
    ] },
    heading(3, "ブロック"),
    callout("note", "ここにノート・補足事項を記述します。"),
    callout("tip", "ここに有用な Tips やヒントを記述します。"),
    callout("warning", "ここに注意事項・警告を記述します。"),
    callout("danger", "ここに重大な危険・破壊的変更への注意を記述します。"),
    paragraph(text("コードは "), text("tomet export .", [{ type: "code" }]), text(" で書き出せます。")),
    { type: "codeBlock", attrs: { language: "typescript" }, content: [text('console.log("Hello, Tomet!");')] },
  ],
};
