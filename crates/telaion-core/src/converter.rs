use serde::{Deserialize, Serialize};
use tomet_ast::{
    Block, Document, Element, Inline, List, ListItem, Name, Paragraph, RawText, Section, Sigil,
    Span, Text,
};

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

/// Convert Tomet markup source string to ProseMirror JSON document
pub fn tomet_to_prosemirror(source: &str) -> anyhow::Result<ProseMirrorDoc> {
    let doc = tomet_parser::parse_document(source)
        .map_err(|e| anyhow::anyhow!("Failed to parse Tomet document: {e}"))?;

    let mut content = Vec::new();

    for block in &doc.blocks {
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

                // Extract block or inlines from item.element.content
                if let Some(ref blocks) = item.element.content {
                    for b in blocks {
                        if let Some(n) = block_to_pm_node(b) {
                            item_content.push(n);
                        }
                    }
                }

                // If element has no block content, check if it's empty
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
            let name_str = el
                .sigil
                .name()
                .map(|n| n.to_string())
                .unwrap_or_else(|| "element".to_string());

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
                    "sigil": format!("{:?}", el.sigil),
                })),
                content: Vec::new(),
                marks: Vec::new(),
            })
        }
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
                let name = el.sigil.name().map(|n| n.to_string()).unwrap_or_default();
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
                    let item_element = Element {
                        sigil: Sigil::Bare,
                        placement: tomet_ast::Placement::Block,
                        args: None,
                        content: Some(item_blocks),
                        value: None,
                        id: None,
                        connects: Vec::new(),
                        span: Span::dummy(),
                    };
                    items.push(ListItem {
                        element: item_element,
                        sublist: None,
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
                let callout_el = Element {
                    sigil: Sigil::Named(Name::bare(variant)),
                    placement: tomet_ast::Placement::Block,
                    args: None,
                    content: Some(inner_blocks),
                    value: None,
                    id: None,
                    connects: Vec::new(),
                    span: Span::dummy(),
                };
                blocks.push(Block::Element(callout_el));
            }
            _ => {}
        }
    }

    let tomet_doc = Document::new(blocks, Span::dummy());
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
                let el = Element {
                    sigil: Sigil::Named(Name::bare("strong")),
                    placement: tomet_ast::Placement::Inline,
                    args: None,
                    content: Some(vec![Block::Paragraph(child_paragraph)]),
                    value: None,
                    id: None,
                    connects: Vec::new(),
                    span: Span::dummy(),
                };
                inlines.push(Inline::Element(el));
            } else if is_italic {
                let child_paragraph = Paragraph::new(
                    vec![Inline::Text(Text::new(text, Span::dummy()))],
                    Span::dummy(),
                );
                let el = Element {
                    sigil: Sigil::Named(Name::bare("em")),
                    placement: tomet_ast::Placement::Inline,
                    args: None,
                    content: Some(vec![Block::Paragraph(child_paragraph)]),
                    value: None,
                    id: None,
                    connects: Vec::new(),
                    span: Span::dummy(),
                };
                inlines.push(Inline::Element(el));
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
}
