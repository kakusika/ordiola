use std::path::Path;
use serde::{Deserialize, Serialize};
use typst::diag::{Severity, SourceDiagnostic};
use typst_layout::PagedDocument;
use typst::WorldExt;
use typst_svg::{svg, SvgOptions};

use crate::world::TypstWorld;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypstDiagnostic {
    pub severity: String,
    pub message: String,
    pub line: usize,
    pub column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypstCompileOutput {
    pub success: bool,
    pub pages: Vec<String>,
    pub diagnostics: Vec<TypstDiagnostic>,
    pub num_pages: usize,
}

pub fn compile_source(
    source: &str,
    relative_path: Option<&str>,
    vault_root: Option<&Path>,
) -> TypstCompileOutput {
    let world = TypstWorld::new(source.to_string(), relative_path, vault_root);
    let warned = typst::compile::<PagedDocument>(&world);

    let mut diagnostics = Vec::new();
    for warning in warned.warnings {
        diagnostics.push(convert_diagnostic(&world, &warning));
    }

    match warned.output {
        Ok(document) => {
            let opts = SvgOptions::default();
            let mut pages = Vec::with_capacity(document.pages().len());
            for page in document.pages() {
                let svg_str = svg(page, &opts);
                pages.push(svg_str);
            }
            let num_pages = pages.len();
            TypstCompileOutput {
                success: true,
                pages,
                diagnostics,
                num_pages,
            }
        }
        Err(errors) => {
            for err in errors {
                diagnostics.push(convert_diagnostic(&world, &err));
            }
            TypstCompileOutput {
                success: false,
                pages: Vec::new(),
                diagnostics,
                num_pages: 0,
            }
        }
    }
}

pub fn export_pdf(
    source: &str,
    relative_path: Option<&str>,
    vault_root: Option<&Path>,
    output_path: &Path,
) -> Result<(), String> {
    let world = TypstWorld::new(source.to_string(), relative_path, vault_root);
    let warned = typst::compile::<PagedDocument>(&world);

    match warned.output {
        Ok(document) => {
            let pdf_bytes = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
                .map_err(|errs| {
                    errs.into_iter()
                        .map(|e| e.message.to_string())
                        .collect::<Vec<_>>()
                        .join("\n")
                })?;

            if let Some(parent) = output_path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            std::fs::write(output_path, pdf_bytes).map_err(|e| e.to_string())?;
            Ok(())
        }
        Err(errors) => {
            let msg = errors
                .into_iter()
                .map(|e| e.message.to_string())
                .collect::<Vec<_>>()
                .join("\n");
            Err(format!("Typst compilation failed:\n{msg}"))
        }
    }
}

fn convert_diagnostic(world: &TypstWorld, diag: &SourceDiagnostic) -> TypstDiagnostic {
    let severity = match diag.severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
    }
    .to_string();

    let mut line = 1;
    let mut column = 1;
    let mut end_line = 1;
    let mut end_column = 1;

    let source = world.main_source();
    if let Some(range) = world.range(diag.span) {
        if let Some((l, c)) = source.lines().byte_to_line_column(range.start) {
            line = l + 1;
            column = c + 1;
        }
        if let Some((el, ec)) = source.lines().byte_to_line_column(range.end) {
            end_line = el + 1;
            end_column = ec + 1;
        } else {
            end_line = line;
            end_column = column;
        }
    }

    TypstDiagnostic {
        severity,
        message: diag.message.to_string(),
        line,
        column,
        end_line,
        end_column,
    }
}
