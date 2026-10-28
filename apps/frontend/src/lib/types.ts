export interface FileEntry {
  name: string;
  path: string;
  relative_path: string;
  is_dir: boolean;
  extension?: string | null;
  has_children: boolean;
  children?: FileEntry[] | null;
  isExpanded?: boolean;
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

export interface InlineAction {
  type: "new-file" | "new-folder" | "rename";
  parentPath: string; // relative path of parent dir ("" for root)
  targetEntry?: FileEntry; // the entry being renamed
}

export interface ContextMenuState {
  visible: boolean;
  x: number;
  y: number;
  entry: FileEntry | null; // null represents background context menu
}

export interface DeleteConfirmState {
  visible: boolean;
  entry: FileEntry;
}

