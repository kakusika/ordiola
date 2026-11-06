pub mod compiler;
pub mod world;

pub use compiler::{compile_source, export_pdf, TypstCompileOutput, TypstDiagnostic};
pub use world::TypstWorld;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_simple_typst() {
        let source = r#"
= Hello, Typst!

This is a live preview test from *Ordiola*.

$ E = m c^2 $
"#;
        let output = compile_source(source, None, None);
        assert!(output.success, "Compilation should succeed");
        assert_eq!(output.num_pages, 1);
        assert!(!output.pages.is_empty());
        assert!(output.pages[0].contains("<svg"));
    }

    #[test]
    fn test_compile_with_error() {
        let source = r#"
= Document
#unknown_function_that_does_not_exist()
"#;
        let output = compile_source(source, None, None);
        assert!(!output.success, "Compilation should fail for unknown function");
        assert!(!output.diagnostics.is_empty());
        assert_eq!(output.diagnostics[0].severity, "error");
    }
}
