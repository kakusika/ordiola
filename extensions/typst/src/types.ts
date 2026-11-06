export interface TypstDiagnostic {
  severity: "error" | "warning";
  message: string;
  line: number;
  column: number;
  end_line: number;
  end_column: number;
}

export interface TypstCompileOutput {
  success: boolean;
  pages: string[]; // SVG string for each page
  diagnostics: TypstDiagnostic[];
  num_pages: number;
}

export interface TypstPreviewState {
  zoom: number; // 1.0 = 100%
  fitWidth: boolean;
  currentPage: number;
}
