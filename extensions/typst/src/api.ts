import type { TypstCompileOutput } from "./types";

const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export async function compileTypst(
  source: string,
  relativePath?: string
): Promise<TypstCompileOutput> {
  if (isTauri) {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      return await invoke<TypstCompileOutput>("compile_typst", {
        source,
        relativePath: relativePath || null,
      });
    } catch (e: any) {
      return {
        success: false,
        pages: [],
        diagnostics: [
          {
            severity: "error",
            message: String(e?.message || e),
            line: 1,
            column: 1,
            end_line: 1,
            end_column: 1,
          },
        ],
        num_pages: 0,
      };
    }
  }

  // Fallback mock SVG for browser preview dev
  const mockSvg = `
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 595 842" width="595" height="842" class="typst-page">
  <text x="50" y="80" font-family="serif" font-size="24" font-weight="bold" fill="#111827">Typst Preview (Browser Mock)</text>
  <text x="50" y="120" font-family="serif" font-size="14" fill="#374151">Tauri desktop environment provides real-time Typst 0.15 compilation.</text>
  <line x1="50" y1="140" x2="545" y2="140" stroke="#e5e7eb" stroke-width="1" />
  <text x="50" y="170" font-family="monospace" font-size="12" fill="#4b5563">Source preview length: ${source.length} characters</text>
</svg>
`;

  return {
    success: true,
    pages: [mockSvg],
    diagnostics: [],
    num_pages: 1,
  };
}

export async function exportTypstPdf(
  source: string,
  relativePath: string,
  outputPath: string
): Promise<void> {
  if (!isTauri) {
    throw new Error("PDF export is only available in the desktop application");
  }
  const { invoke } = await import("@tauri-apps/api/core");
  await invoke("export_typst_pdf", {
    source,
    relativePath,
    outputPath,
  });
}
