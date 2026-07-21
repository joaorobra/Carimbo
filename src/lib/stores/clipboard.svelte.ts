// Clipboard-history view state. Mirrors the backend and refreshes on the
// `clipboard:new-entry` event.

import { api } from "../api";
import { Events, onEvent } from "../events";
import { retry } from "../retry";
import type { ClipEntry, Folder, TransformKind } from "../types";

class ClipboardStore {
  entries = $state<ClipEntry[]>([]);
  folders = $state<Folder[]>([]);
  query = $state("");
  loading = $state(false);
  /** null = "All", otherwise a clipboard-folder id filter. */
  activeFolderId = $state<string | null>(null);
  #initialized = false;

  async init(): Promise<void> {
    if (this.#initialized) return;
    // Retry the first load past a cold-start race, and latch #initialized only
    // after it succeeds — otherwise a transient backend-not-ready failure leaves
    // the history empty until a manual F5.
    await retry(() => this.refresh());
    this.#initialized = true;
    await onEvent(Events.ClipboardNew, () => this.refresh());
  }

  async refresh(): Promise<void> {
    this.loading = true;
    try {
      const q = this.query.trim();
      const [entries, folders] = await Promise.all([
        q ? api.searchClips(q) : api.listClips(),
        api.listFolders("clipboard"),
      ]);
      this.entries = entries;
      this.folders = folders;
    } finally {
      this.loading = false;
    }
  }

  /** Entries visible given the active folder filter. */
  get visible(): ClipEntry[] {
    if (this.activeFolderId === null) return this.entries;
    return this.entries.filter((e) => e.folderId === this.activeFolderId);
  }

  async setQuery(q: string): Promise<void> {
    this.query = q;
    await this.refresh();
  }

  setActiveFolder(id: string | null): void {
    this.activeFolderId = id;
  }

  async togglePin(entry: ClipEntry): Promise<void> {
    await api.setClipPinned(entry.id, !entry.isPinned);
    await this.refresh();
  }

  async setFolder(id: string, folderId: string | null): Promise<void> {
    await api.setClipFolder(id, folderId);
    await this.refresh();
  }

  async createFolder(name: string): Promise<Folder> {
    const folder = await api.createFolder(name.trim(), "clipboard", null);
    await this.refresh();
    return folder;
  }

  async remove(id: string): Promise<void> {
    await api.deleteClip(id);
    await this.refresh();
  }

  async copy(id: string, transform?: TransformKind): Promise<void> {
    await api.copyClip(id, transform);
  }

  async promote(id: string, name?: string | null): Promise<string> {
    const snippet = await api.promoteClipToSnippet(id, name);
    return snippet.name;
  }
}

export const clipboardStore = new ClipboardStore();
