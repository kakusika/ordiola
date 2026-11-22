use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tomet_ast::{
    Block, Data, Declaration, Element, Entry, File, Inline, List, ListItem, Name, Paragraph,
    Placement, RawText, Section, Text, Tuple, Value,
};
use tomet_span::Span;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct TometMetadata {
    pub id: Option<String>,
    pub title: Option<String>,
    pub lang: Option<String>,
    #[serde(default)]
    pub custom: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct TometConfig {
    pub export_type: Option<String>,
    pub export_path: Option<String>,
}

fn scalar_text(value: &Value) -> Option<String> {
    value
        .as_scalar()
        .and_then(|s| s.text())
        .map(|s| s.into_owned())
}

fn extract_meta_from_tuple(tuple: &Tuple, meta: &mut TometMetadata) {
    for entry in tuple {
        let Some(key) = entry.name() else { continue };
        let Some(val) = scalar_text(entry.value()) else {
            continue;
        };
        match key {
            "id" => meta.id = Some(val),
            "title" => meta.title = Some(val),
            "lang" => meta.lang = Some(val),
            _ if !key.is_empty() => {
                meta.custom.insert(key.to_string(), val);
            }
            _ => {}
        }
    }
}

/// Extract metadata fields from a Tomet document's header declarations (`::id(...)`, `::meta(...)`).
pub fn extract_tomet_metadata(source: &str) -> TometMetadata {
    let mut meta = TometMetadata::default();
    let Ok(file) = tomet_ast_parser::parse_file(source) else {
        return meta;
    };

    for decl in &file.header {
        if decl.name.is_bare_named("id") {
            if let Some(val) = decl
                .data
                .args
                .as_ref()
                .and_then(|a| a.get("0"))
                .and_then(scalar_text)
            {
                meta.id = Some(val);
            }
        } else if decl.name.is_bare_named("meta") {
            if let Some(ref args) = decl.data.args {
                extract_meta_from_tuple(args, &mut meta);
            }
            if let Some(ref group) = decl.data.value {
                for entry in &group.0 {
                    if let Entry::Pair(key, value) = entry {
                        if let Some(val) = scalar_text(value) {
                            match key.as_str() {
                                "id" => meta.id = Some(val),
                                "title" => meta.title = Some(val),
                                "lang" => meta.lang = Some(val),
                                _ if !key.is_empty() => {
                                    meta.custom.insert(key.clone(), val);
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
    }
    meta
}

fn format_meta_declaration(meta: &TometMetadata) -> Option<String> {
    if meta.title.is_none() && meta.lang.is_none() && meta.custom.is_empty() {
        return None;
    }
    let mut lines = vec!["::meta(".to_string()];
    if let Some(ref title) = meta.title {
        let escaped = title.replace('\\', "\\\\").replace('"', "\\\"");
        lines.push(format!("  title: \"{escaped}\""));
    }
    if let Some(ref lang) = meta.lang {
        lines.push(format!("  lang: {lang}"));
    }
    for (k, v) in &meta.custom {
        let escaped = v.replace('\\', "\\\\").replace('"', "\\\"");
        lines.push(format!("  {k}: \"{escaped}\""));
    }
    lines.push(")".to_string());
    Some(lines.join("\n"))
}

fn find_header_decl<'a>(header: &'a [Declaration], name: &str) -> Option<&'a Declaration> {
    header.iter().find(|d| d.name.is_bare_named(name))
}

/// Inject or replace the `::id(...)` and `::meta(...)` declarations in a Tomet document string.
pub fn inject_tomet_metadata(source: &str, meta: &TometMetadata) -> String {
    let id_block = meta.id.as_ref().map(|id| format!("::id({id})"));
    let meta_block = format_meta_declaration(meta);

    let Ok(file) = tomet_ast_parser::parse_file(source) else {
        let mut prefix = Vec::new();
        if let Some(id_str) = id_block {
            prefix.push(id_str);
        }
        if let Some(meta_str) = meta_block {
            prefix.push(meta_str);
        }
        if prefix.is_empty() {
            return source.to_string();
        }
        return format!("{}\n\n{source}", prefix.join("\n"));
    };

    let existing_id = find_header_decl(&file.header, "id");
    let existing_meta = find_header_decl(&file.header, "meta");

    // Collect (start, end, replacement) edits in source order, then apply from back to front.
    let mut edits: Vec<(usize, usize, String)> = Vec::new();
    let mut to_append: Vec<String> = Vec::new();

    match (existing_id, id_block) {
        (Some(decl), Some(rep)) => edits.push((decl.span.start, decl.span.end, rep)),
        (Some(decl), None) => {
            let end = if source.as_bytes().get(decl.span.end) == Some(&b'\n') {
                decl.span.end + 1
            } else {
                decl.span.end
            };
            edits.push((decl.span.start, end, String::new()));
        }
        (None, Some(rep)) => to_append.push(rep),
        (None, None) => {}
    }

    match (existing_meta, meta_block) {
        (Some(decl), Some(rep)) => edits.push((decl.span.start, decl.span.end, rep)),
        (Some(decl), None) => {
            let end = if source.as_bytes().get(decl.span.end) == Some(&b'\n') {
                decl.span.end + 1
            } else {
                decl.span.end
            };
            edits.push((decl.span.start, end, String::new()));
        }
        (None, Some(rep)) => to_append.push(rep),
        (None, None) => {}
    }

    let mut result = source.to_string();
    edits.sort_by_key(|a| std::cmp::Reverse(a.0));
    for (start, end, rep) in edits {
        result.replace_range(start..end, &rep);
    }

    if !to_append.is_empty() {
        let joined = to_append.join("\n");
        if file.header.is_empty() {
            result = format!("{joined}\n\n{result}");
        } else {
            // Re-parse after in-place replacements to insert cleanly at the end of header.
            let insert_at = tomet_ast_parser::parse_file(&result)
                .ok()
                .and_then(|f| f.header.last().map(|d| d.span.end))
                .unwrap_or(0);
            result.insert_str(insert_at, &format!("\n{joined}"));
        }
    }

    result
}

/// Extract export configuration from `::export(...)` header declaration.
pub fn extract_tomet_config(source: &str) -> TometConfig {
    let mut config = TometConfig::default();
    let Ok(file) = tomet_ast_parser::parse_file(source) else {
        return config;
    };

    if let Some(decl) = find_header_decl(&file.header, "export") {
        if let Some(ref args) = decl.data.args {
            config.export_type = args.get("type").and_then(scalar_text);
            config.export_path = args.get("path").and_then(scalar_text);
        }
    }
    config
}

/// ProseMirror Document Node structure (schema: "doc")
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProseMirrorDoc {
    #[serde(rename = "type")]
    pub node_type: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub content: Vec<ProseMirrorNode>,
}

impl Default for ProseMirrorDoc {
    fn default() -> Self {
        Self {
            node_type: "doc".to_string(),
            content: Vec::new(),
        }
    }
}

/// Generic ProseMirror Node representation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProseMirrorNode {
    #[serde(rename = "type")]
    pub node_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attrs: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub content: Vec<ProseMirrorNode>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub marks: Vec<ProseMirrorMark>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProseMirrorMark {
    #[serde(rename = "type")]
    pub mark_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attrs: Option<serde_json::Value>,
}

impl ProseMirrorNode {
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            node_type: "text".to_string(),
            text: Some(text.into()),
            attrs: None,
            content: Vec::new(),
            marks: Vec::new(),
        }
    }

    pub fn text_with_marks(text: impl Into<String>, marks: Vec<ProseMirrorMark>) -> Self {
        Self {
            node_type: "text".to_string(),
            text: Some(text.into()),
            attrs: None,
            content: Vec::new(),
            marks,
        }
    }

    pub fn paragraph(content: Vec<ProseMirrorNode>) -> Self {
        Self {
            node_type: "paragraph".to_string(),
            text: None,
            attrs: None,
            content,
            marks: Vec::new(),
        }
    }

    pub fn heading(level: usize, content: Vec<ProseMirrorNode>) -> Self {
        Self {
            node_type: "heading".to_string(),
            text: None,
            attrs: Some(serde_json::json!({ "level": level })),
            content,
            marks: Vec::new(),
        }
    }

    pub fn bullet_list(content: Vec<ProseMirrorNode>) -> Self {
        Self {
            node_type: "bulletList".to_string(),
            text: None,
            attrs: None,
            content,
            marks: Vec::new(),
        }
    }

    pub fn ordered_list(content: Vec<ProseMirrorNode>) -> Self {
        Self {
            node_type: "orderedList".to_string(),
            text: None,
            attrs: None,
            content,
            marks: Vec::new(),
        }
    }

    pub fn list_item(content: Vec<ProseMirrorNode>) -> Self {
        Self {
            node_type: "listItem".to_string(),
            text: None,
            attrs: None,
            content,
            marks: Vec::new(),
        }
    }

    pub fn code_block(code: String, lang: Option<String>) -> Self {
        let text_node = ProseMirrorNode::text(code);
        Self {
            node_type: "codeBlock".to_string(),
            text: None,
            attrs: lang.map(|l| serde_json::json!({ "language": l })),
            content: vec![text_node],
            marks: Vec::new(),
        }
    }
}

fn make_element(name: &str, placement: Placement, content: Vec<Block>) -> Element {
    Element {
        name: Name::bare(name),
        placement,
        data: Data::default(),
        content: Some(content),
        id: None,
        connects: Vec::new(),
        span: Span::dummy(),
    }
}

/// Convert Tomet markup source string to ProseMirror JSON document
pub fn tomet_to_prosemirror(source: &str) -> anyhow::Result<ProseMirrorDoc> {
    let doc = tomet_ast_parser::parse_file(source)
        .map_err(|e| anyhow::anyhow!("Failed to parse Tomet document: {e}"))?;

    let mut content = Vec::new();

    for block in &doc.body {
        match block {
            Block::Section(sec) => {
                let title_nodes = inlines_to_pm_nodes(&sec.title);
                content.push(ProseMirrorNode::heading(sec.level, title_nodes));
                for child in &sec.blocks {
                    if let Some(node) = block_to_pm_node(child) {
                        content.push(node);
                    }
                }
            }
            _ => {
                if let Some(node) = block_to_pm_node(block) {
                    content.push(node);
                }
            }
        }
    }

    Ok(ProseMirrorDoc {
        node_type: "doc".to_string(),
        content,
    })
}

fn block_to_pm_node(block: &Block) -> Option<ProseMirrorNode> {
    match block {
        Block::Paragraph(p) => {
            let inlines = inlines_to_pm_nodes(&p.content);
            Some(ProseMirrorNode::paragraph(inlines))
        }
        Block::Section(s) => {
            let inlines = inlines_to_pm_nodes(&s.title);
            Some(ProseMirrorNode::heading(s.level, inlines))
        }
        Block::List(list) => {
            let mut item_nodes = Vec::new();
            for item in &list.items {
                let mut item_content = Vec::new();

                for b in &item.content {
                    if let Some(n) = block_to_pm_node(b) {
                        item_content.push(n);
                    }
                }

                // If item has no block content, ensure an empty paragraph for ProseMirror
                if item_content.is_empty() {
                    item_content.push(ProseMirrorNode::paragraph(Vec::new()));
                }

                // Handle nested sublist if present
                if let Some(ref sublist) = item.sublist {
                    if let Some(sublist_node) = block_to_pm_node(&Block::List(sublist.clone())) {
                        item_content.push(sublist_node);
                    }
                }

                item_nodes.push(ProseMirrorNode::list_item(item_content));
            }

            if list.ordered {
                Some(ProseMirrorNode::ordered_list(item_nodes))
            } else {
                Some(ProseMirrorNode::bullet_list(item_nodes))
            }
        }
        Block::Element(el) => {
            let name_str = el.name.to_string();

            // Check for callout or note element
            if name_str == "callout"
                || name_str == "note"
                || name_str == "info"
                || name_str == "warning"
            {
                let mut inner_blocks = Vec::new();
                if let Some(ref blocks) = el.content {
                    for b in blocks {
                        if let Some(n) = block_to_pm_node(b) {
                            inner_blocks.push(n);
                        }
                    }
                }
                if inner_blocks.is_empty() {
                    inner_blocks.push(ProseMirrorNode::paragraph(Vec::new()));
                }
                return Some(ProseMirrorNode {
                    node_type: "callout".to_string(),
                    text: None,
                    attrs: Some(serde_json::json!({ "type": name_str })),
                    content: inner_blocks,
                    marks: Vec::new(),
                });
            }

            // Check for code block element
            if name_str == "code" {
                let mut code_text = String::new();
                if let Some(ref blocks) = el.content {
                    for b in blocks {
                        if let Block::Paragraph(p) = b {
                            for inline in &p.content {
                                match inline {
                                    Inline::Text(t) => code_text.push_str(&t.value),
                                    Inline::Raw(r) => code_text.push_str(&r.value),
                                    Inline::LineBreak(_) => code_text.push('\n'),
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                return Some(ProseMirrorNode::code_block(code_text, None));
            }

            Some(ProseMirrorNode {
                node_type: "tomet_element".to_string(),
                text: None,
                attrs: Some(serde_json::json!({
                    "name": name_str,
                })),
                content: Vec::new(),
                marks: Vec::new(),
            })
        }
        Block::Hole(_) | Block::TypeSeat(_) | Block::Interp(_) => None,
    }
}

fn inlines_to_pm_nodes(inlines: &[Inline]) -> Vec<ProseMirrorNode> {
    let mut nodes = Vec::new();
    for inline in inlines {
        match inline {
            Inline::Text(t) => {
                nodes.push(ProseMirrorNode::text(&t.value));
            }
            Inline::Raw(r) => {
                nodes.push(ProseMirrorNode::text_with_marks(
                    &r.value,
                    vec![ProseMirrorMark {
                        mark_type: "code".to_string(),
                        attrs: None,
                    }],
                ));
            }
            Inline::SoftBreak(_) => {
                nodes.push(ProseMirrorNode::text(" "));
            }
            Inline::LineBreak(_) => {
                nodes.push(ProseMirrorNode::text("\n"));
            }
            Inline::Element(el) => {
                let name = el.name.to_string();
                match name.as_str() {
                    "strong" | "bold" => {
                        let text = extract_inlines_text(el);
                        nodes.push(ProseMirrorNode::text_with_marks(
                            text,
                            vec![ProseMirrorMark {
                                mark_type: "bold".to_string(),
                                attrs: None,
                            }],
                        ));
                    }
                    "em" | "italic" => {
                        let text = extract_inlines_text(el);
                        nodes.push(ProseMirrorNode::text_with_marks(
                            text,
                            vec![ProseMirrorMark {
                                mark_type: "italic".to_string(),
                                attrs: None,
                            }],
                        ));
                    }
                    "code" => {
                        let text = extract_inlines_text(el);
                        nodes.push(ProseMirrorNode::text_with_marks(
                            text,
                            vec![ProseMirrorMark {
                                mark_type: "code".to_string(),
                                attrs: None,
                            }],
                        ));
                    }
                    "strike" => {
                        let text = extract_inlines_text(el);
                        nodes.push(ProseMirrorNode::text_with_marks(
                            text,
                            vec![ProseMirrorMark {
                                mark_type: "strike".to_string(),
                                attrs: None,
                            }],
                        ));
                    }
                    _ => {
                        nodes.push(ProseMirrorNode {
                            node_type: "tomet_inline".to_string(),
                            text: None,
                            attrs: Some(serde_json::json!({
                                "name": name,
                            })),
                            content: Vec::new(),
                            marks: Vec::new(),
                        });
                    }
                }
            }
            Inline::Hole(_) | Inline::TypeSeat(_) | Inline::Interp(_) => {}
        }
    }
    nodes
}

fn extract_inlines_text(el: &Element) -> String {
    let mut text = String::new();
    if let Some(ref blocks) = el.content {
        for b in blocks {
            if let Block::Paragraph(p) = b {
                for inline in &p.content {
                    match inline {
                        Inline::Text(t) => text.push_str(&t.value),
                        Inline::Raw(r) => text.push_str(&r.value),
                        Inline::Element(child_el) => {
                            text.push_str(&extract_inlines_text(child_el));
                        }
                        Inline::SoftBreak(_) => text.push(' '),
                        Inline::LineBreak(_) => text.push('\n'),
                        Inline::Hole(_) | Inline::TypeSeat(_) | Inline::Interp(_) => {}
                    }
                }
            }
        }
    }
    text
}

/// Convert ProseMirror JSON document back to Tomet markup source string
pub fn prosemirror_to_tomet(doc: &ProseMirrorDoc) -> anyhow::Result<String> {
    let mut blocks = Vec::new();

    for node in &doc.content {
        match node.node_type.as_str() {
            "heading" => {
                let level = node
                    .attrs
                    .as_ref()
                    .and_then(|a| a.get("level"))
                    .and_then(|l| l.as_u64())
                    .unwrap_or(1) as usize;
                let title = pm_nodes_to_inlines(&node.content);
                blocks.push(Block::Section(Section::new(level, title, Span::dummy())));
            }
            "paragraph" => {
                let inlines = pm_nodes_to_inlines(&node.content);
                blocks.push(Block::Paragraph(Paragraph::new(inlines, Span::dummy())));
            }
            "bulletList" | "orderedList" => {
                let ordered = node.node_type == "orderedList";
                let mut items = Vec::new();
                for item_node in &node.content {
                    let mut item_blocks = Vec::new();
                    for child in &item_node.content {
                        if child.node_type.as_str() == "paragraph" {
                            let inlines = pm_nodes_to_inlines(&child.content);
                            item_blocks
                                .push(Block::Paragraph(Paragraph::new(inlines, Span::dummy())));
                        }
                    }
                    items.push(ListItem {
                        name: None,
                        data: Data::default(),
                        content: item_blocks,
                        id: None,
                        connects: Vec::new(),
                        sublist: None,
                        span: Span::dummy(),
                    });
                }
                blocks.push(Block::List(List::new(ordered, items, Span::dummy())));
            }
            "callout" => {
                let variant = node
                    .attrs
                    .as_ref()
                    .and_then(|a| a.get("type"))
                    .and_then(|t| t.as_str())
                    .unwrap_or("info");
                let mut inner_blocks = Vec::new();
                for child in &node.content {
                    if child.node_type == "paragraph" {
                        let inlines = pm_nodes_to_inlines(&child.content);
                        inner_blocks.push(Block::Paragraph(Paragraph::new(inlines, Span::dummy())));
                    }
                }
                blocks.push(Block::Element(make_element(
                    variant,
                    Placement::Block,
                    inner_blocks,
                )));
            }
            "codeBlock" => {
                let code_text = node
                    .content
                    .iter()
                    .filter_map(|c| c.text.as_deref())
                    .collect::<Vec<_>>()
                    .join("");
                let raw_content = vec![Block::Paragraph(Paragraph::new(
                    vec![Inline::Raw(RawText::new(code_text, Span::dummy()))],
                    Span::dummy(),
                ))];
                blocks.push(Block::Element(make_element(
                    "code",
                    Placement::Block,
                    raw_content,
                )));
            }
            _ => {}
        }
    }

    let tomet_doc = File::new(Vec::new(), blocks, Span::dummy());
    Ok(tomet_printer::document_to_tm(&tomet_doc))
}

fn pm_nodes_to_inlines(nodes: &[ProseMirrorNode]) -> Vec<Inline> {
    let mut inlines = Vec::new();
    for node in nodes {
        if let Some(ref text) = node.text {
            // Check marks
            let is_bold = node.marks.iter().any(|m| m.mark_type == "bold");
            let is_italic = node.marks.iter().any(|m| m.mark_type == "italic");
            let is_code = node.marks.iter().any(|m| m.mark_type == "code");

            if is_code {
                inlines.push(Inline::Raw(RawText::new(text, Span::dummy())));
            } else if is_bold {
                let child_paragraph = Paragraph::new(
                    vec![Inline::Text(Text::new(text, Span::dummy()))],
                    Span::dummy(),
                );
                inlines.push(Inline::Element(make_element(
                    "strong",
                    Placement::Inline,
                    vec![Block::Paragraph(child_paragraph)],
                )));
            } else if is_italic {
                let child_paragraph = Paragraph::new(
                    vec![Inline::Text(Text::new(text, Span::dummy()))],
                    Span::dummy(),
                );
                inlines.push(Inline::Element(make_element(
                    "em",
                    Placement::Inline,
                    vec![Block::Paragraph(child_paragraph)],
                )));
            } else {
                inlines.push(Inline::Text(Text::new(text, Span::dummy())));
            }
        }
    }
    inlines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tomet_to_prosemirror_heading_and_paragraph() {
        let input = "=[ Title ]\n\nHello world!\n";
        let pm_doc = tomet_to_prosemirror(input).expect("parse succeeded");
        assert_eq!(pm_doc.node_type, "doc");
        assert!(pm_doc.content.len() >= 2);
    }

    #[test]
    fn test_tomet_to_prosemirror_bold_and_list() {
        let input = "Here is **bold text** in paragraph.\n\n- item 1\n- item 2\n";
        let pm_doc = tomet_to_prosemirror(input).expect("parse succeeded");
        assert_eq!(pm_doc.node_type, "doc");
        assert_eq!(pm_doc.content.len(), 2);

        // First block: paragraph with bold mark
        let p = &pm_doc.content[0];
        assert_eq!(p.node_type, "paragraph");
        let has_bold = p
            .content
            .iter()
            .any(|n| n.marks.iter().any(|m| m.mark_type == "bold"));
        assert!(has_bold, "Expected bold mark in paragraph content");

        // Second block: bulletList
        let list = &pm_doc.content[1];
        assert_eq!(list.node_type, "bulletList");
        assert_eq!(list.content.len(), 2);
    }

    #[test]
    fn test_roundtrip_tomet_prosemirror() {
        let input = "=[ Welcome ]\n\nSimple text paragraph.\n";
        let pm_doc = tomet_to_prosemirror(input).expect("parse succeeded");
        let output = prosemirror_to_tomet(&pm_doc).expect("serialization succeeded");
        assert!(output.contains("Welcome"));
        assert!(output.contains("Simple text paragraph."));
    }

    #[test]
    fn test_parse_real_readme_tmt() {
        let content = std::fs::read_to_string("../../tmtroot/readme.tmt").unwrap();
        let res = tomet_to_prosemirror(&content);
        assert!(res.is_ok());
    }

    #[test]
    fn test_extract_and_inject_metadata() {
        let source = "::id(test-id)\n::meta(\n  lang: ja\n)\n\n=[ Title ]\n";
        let meta = extract_tomet_metadata(source);
        assert_eq!(meta.id.as_deref(), Some("test-id"));
        assert_eq!(meta.lang.as_deref(), Some("ja"));

        let mut updated = meta.clone();
        updated.title = Some("New Title".to_string());
        updated
            .custom
            .insert("author".to_string(), "Alice".to_string());

        let injected = inject_tomet_metadata(source, &updated);
        assert!(injected.contains("::id(test-id)"));
        assert!(injected.contains("title: \"New Title\""));
        assert!(injected.contains("author: \"Alice\""));
        let reparsed = extract_tomet_metadata(&injected);
        assert_eq!(reparsed, updated);
    }

    #[test]
    fn test_extract_config() {
        let source = "::export(\n  type: commonmark\n  path: \"README.md\"\n)\n";
        let cfg = extract_tomet_config(source);
        assert_eq!(cfg.export_type.as_deref(), Some("commonmark"));
        assert_eq!(cfg.export_path.as_deref(), Some("README.md"));
    }
}
