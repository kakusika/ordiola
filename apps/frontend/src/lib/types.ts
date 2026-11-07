export interface FileEntry {
  name: string;
  path: string;
  relative_path: string;
  is_dir: boolean;
  extension?: string | null;
  has_children: boolean;
  /** The entries inside a directory; not set until the directory has been listed. */
  children?: FileEntry[] | null;
  /** The directory is being listed. */
  isLoading?: boolean;
}

export interface VaultEntry {
  name: string;
  path: string;
  last_opened: number;
}

export interface VaultState {
  current_path: string;
  current_name: string;
  recent_vaults: VaultEntry[];
}

export interface DeleteConfirmState {
  visible: boolean;
  entry: FileEntry;
}

export interface EditorTab {
  id: string;
  path: string;
  title: string;
  isModified: boolean;
  isTmt: boolean;
  isTypst: boolean;
  rawContent: string;
  pmDoc?: any;
  editMode: "source" | "hybrid" | "full" | "kanban";
}



/** The `::meta` of a Tomet document, as the metadata inspector edits it. */
export interface TometDocMeta {
  id: string;
  title: string;
  lang: string;
  custom: Record<string, string>;
}

/** The `::config` of a Tomet document, as the metadata inspector edits it. */
export interface TometDocConfig {
  export_type: string;
  export_path: string;
}
