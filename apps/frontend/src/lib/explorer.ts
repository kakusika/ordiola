import type { FileEntry } from "./types";

/** What a new entry in the explorer is. */
export type NewEntryKind = "file" | "folder";

/**
 * Paths in the explorer. A node of the tree is identified by the vault-relative path of its entry
 * (`relative_path`, else `path`), with `/` as the separator. The vault root is the empty string.
 */

/** The id of an entry in the tree, and the path every file operation addresses it by. */
export function entryPath(entry: FileEntry): string {
  return entry.relative_path || entry.path;
}

/** The directory a path is in; the empty string for the vault root. */
export function parentPath(path: string): string {
  const at = path.lastIndexOf("/");
  return at > 0 ? path.substring(0, at) : "";
}

export function joinPath(dir: string, name: string): string {
  return dir && dir !== "." ? `${dir}/${name}` : name;
}

/** Every directory that has to be open for `dir` to be seen: its ancestors, then itself. */
export function pathChain(dir: string): string[] {
  const chain: string[] = [];
  let walked = "";
  for (const part of dir.split("/").filter(Boolean)) {
    walked = joinPath(walked, part);
    chain.push(walked);
  }
  return chain;
}

/** The entry at `path` among the loaded entries, or `null`. */
export function findEntry(
  entries: FileEntry[],
  path: string | null,
): FileEntry | null {
  if (!path) return null;
  for (const entry of entries) {
    if (entryPath(entry) === path) return entry;
    if (
      entry.is_dir &&
      entry.children &&
      path.startsWith(entryPath(entry) + "/")
    ) {
      const found = findEntry(entry.children, path);
      if (found) return found;
    }
  }
  return null;
}

/**
 * The entries inside a directory (`""` is the vault root), or `null` when they are not loaded yet.
 */
export function childrenOf(
  entries: FileEntry[],
  dir: string,
): FileEntry[] | null {
  if (!dir || dir === ".") return entries;
  return findEntry(entries, dir)?.children ?? null;
}

/**
 * Where a new file or folder goes when the person did not say: inside the selected directory, or
 * beside the selected file, else at the root.
 */
export function creationDir(
  entries: FileEntry[],
  selectedPath: string | null,
): string {
  const selected = findEntry(entries, selectedPath);
  if (!selected) return "";
  return selected.is_dir
    ? entryPath(selected)
    : parentPath(entryPath(selected));
}

/** The path of `path` after the entry at `from` was moved to `to`; unchanged when it is not inside. */
export function remapPath(path: string, from: string, to: string): string {
  if (path === from) return to;
  return path.startsWith(from + "/") ? to + path.substring(from.length) : path;
}

// ---- The browser mock --------------------------------------------------------------------------
// Without Tauri there is no file system, and the tree is a plain list of entries. These keep it
// behaving like one, so that creating, renaming and moving can be tried in a browser.

export function mockInsert(
  entries: FileEntry[],
  dir: string,
  entry: FileEntry,
): void {
  const target = childrenOf(entries, dir);
  if (target) target.push(entry);
}

export function mockDetach(
  entries: FileEntry[],
  path: string,
): FileEntry | null {
  const list = childrenOf(entries, parentPath(path));
  const at = list?.findIndex((entry) => entryPath(entry) === path) ?? -1;
  return list && at >= 0 ? list.splice(at, 1)[0] : null;
}

/** Gives an entry and everything inside it the path it has after it moved from `from` to `to`. */
export function mockRelocate(entry: FileEntry, from: string, to: string): void {
  entry.relative_path = entry.path = remapPath(entryPath(entry), from, to);
  entry.name = entry.relative_path.substring(
    entry.relative_path.lastIndexOf("/") + 1,
  );
  for (const child of entry.children ?? []) mockRelocate(child, from, to);
}
