use serde::{Deserialize, Serialize};
use tomet_ast::{Block, Document, Inline, Paragraph, Section, Span, Text};

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
        Block::Element(el) => {
            let name_str = el.sigil.name().map(|n| n.to_string()).unwrap_or_else(|| "element".to_string());
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
        Block::List(_) => {
            // TODO: Detailed list mapping
            None
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
                nodes.push(ProseMirrorNode::text(&r.value));
            }
            Inline::SoftBreak(_) => {
                nodes.push(ProseMirrorNode::text(" "));
            }
            Inline::LineBreak(_) => {
                nodes.push(ProseMirrorNode::text("\n"));
            }
            Inline::Element(el) => {
                let name_str = el.sigil.name().map(|n| n.to_string()).unwrap_or_else(|| "inline".to_string());
                nodes.push(ProseMirrorNode {
                    node_type: "tomet_inline".to_string(),
                    text: None,
                    attrs: Some(serde_json::json!({
                        "name": name_str,
                    })),
                    content: Vec::new(),
                    marks: Vec::new(),
                });
            }
        }
    }
    nodes
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
            inlines.push(Inline::Text(Text::new(text, Span::dummy())));
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
}
