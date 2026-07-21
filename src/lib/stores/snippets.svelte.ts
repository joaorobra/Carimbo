// Snippet + folder view state. The backend (SQLite) is the source of truth; this
// store mirrors it and re-fetches on `snippets:changed` / `folders:changed`.

import { api } from "../api";
import { Events, onEvent } from "../events";
import { retry } from "../retry";
import type { Folder, Snippet } from "../types";

class SnippetStore {
  snippets = $state<Snippet[]>([]);
  folders = $state<Folder[]>([]);
  query = $state("");
  loading = $state(false);
  selectedId = $state<string | null>(null);
  /** null = "All", otherwise a folder id filter. */
  activeFolderId = $state<string | null>(null);

  #initialized = false;

  /** Load data and subscribe to change events. Idempotent. */
  async init(): Promise<void> {
    if (this.#initialized) return;
    // Retry the first load: on a cold start the backend may not have finished
    // managing its state yet, so the initial fetch can reject. Only latch
    // #initialized once it succeeds so a transient failure doesn't leave the
    // store permanently empty (the old behaviour that needed a manual F5).
    await retry(() => this.refresh());
    this.#initialized = true;
    await onEvent(Events.SnippetsChanged, () => this.refreshSnippets());
    await onEvent(Events.FoldersChanged, () => this.refreshFolders());
  }

  async refresh(): Promise<void> {
    this.loading = true;
    try {
      await Promise.all([this.refreshSnippets(), this.refreshFolders()]);
    } finally {
      this.loading = false;
    }
  }

  async refreshSnippets(): Promise<void> {
    const q = this.query.trim();
    this.snippets = q ? await api.searchSnippets(q) : await api.listSnippets();
  }

  async refreshFolders(): Promise<void> {
    this.folders = await api.listFolders("snippet");
  }

  async setQuery(q: string): Promise<void> {
    this.query = q;
    await this.refreshSnippets();
  }

  /** Snippets visible given the active folder filter. */
  get visible(): Snippet[] {
    if (this.activeFolderId === null) return this.snippets;
    return this.snippets.filter((s) => s.folderId === this.activeFolderId);
  }

  get selected(): Snippet | null {
    return this.snippets.find((s) => s.id === this.selectedId) ?? null;
  }

  select(id: string | null): void {
    this.selectedId = id;
  }
}

export const snippetStore = new SnippetStore();
