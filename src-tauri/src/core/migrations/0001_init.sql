-- Carimbo initial schema. Sync-ready from day one:
--   * TEXT UUID v4 primary keys (stable across devices)
--   * created_at / updated_at as unix milliseconds UTC
--   * deleted_at tombstones — deletes are ALWAYS soft (UPDATE, never DELETE)
-- Phase-2 cloud sync (LWW by updated_at) plugs in without a schema break.

CREATE TABLE folders (
  id          TEXT PRIMARY KEY,
  name        TEXT NOT NULL,
  kind        TEXT NOT NULL DEFAULT 'snippet',   -- 'snippet' | 'clipboard'
  parent_id   TEXT REFERENCES folders(id),
  sort_order  INTEGER NOT NULL DEFAULT 0,
  created_at  INTEGER NOT NULL,
  updated_at  INTEGER NOT NULL,
  deleted_at  INTEGER
);
CREATE INDEX idx_folders_kind ON folders(kind) WHERE deleted_at IS NULL;

CREATE TABLE snippets (
  id           TEXT PRIMARY KEY,
  folder_id    TEXT REFERENCES folders(id),
  name         TEXT NOT NULL,
  trigger      TEXT,                             -- e.g. ';cpf' (optional)
  body         TEXT NOT NULL,                    -- may contain {date}{time}{clipboard}
  is_favorite  INTEGER NOT NULL DEFAULT 0,
  use_count    INTEGER NOT NULL DEFAULT 0,       -- palette ranking
  last_used_at INTEGER,
  sort_order   INTEGER NOT NULL DEFAULT 0,
  created_at   INTEGER NOT NULL,
  updated_at   INTEGER NOT NULL,
  deleted_at   INTEGER
);
-- A trigger must be unique among LIVE snippets (tombstoned ones don't count).
CREATE UNIQUE INDEX idx_snippets_trigger
  ON snippets(trigger)
  WHERE deleted_at IS NULL AND trigger IS NOT NULL;
CREATE INDEX idx_snippets_folder ON snippets(folder_id) WHERE deleted_at IS NULL;

CREATE TABLE clip_entries (
  id           TEXT PRIMARY KEY,
  kind         TEXT NOT NULL DEFAULT 'text',     -- 'text' | 'image'
  content      TEXT,                             -- text payload
  image_path   TEXT,                             -- PNG on disk (not in DB)
  preview      TEXT NOT NULL,                    -- short display string
  content_hash TEXT NOT NULL,                    -- dedupe re-copies
  source_app   TEXT,                             -- foreground exe name (best effort)
  is_pinned    INTEGER NOT NULL DEFAULT 0,
  folder_id    TEXT REFERENCES folders(id),
  created_at   INTEGER NOT NULL,
  updated_at   INTEGER NOT NULL,
  deleted_at   INTEGER
);
CREATE INDEX idx_clip_created ON clip_entries(created_at DESC) WHERE deleted_at IS NULL;
CREATE INDEX idx_clip_hash ON clip_entries(content_hash);

CREATE TABLE settings (
  key        TEXT PRIMARY KEY,
  value      TEXT NOT NULL,
  updated_at INTEGER NOT NULL
);

-- Full-text search (external-content FTS5) for snippets.
-- unicode61 + remove_diacritics 2 makes pt-BR search accent-insensitive:
-- searching "endereco" matches "endereço".
CREATE VIRTUAL TABLE snippets_fts USING fts5(
  name, body, trigger,
  content='snippets',
  content_rowid='rowid',
  tokenize='unicode61 remove_diacritics 2'
);

-- Keep the FTS index in sync with the base table. Only live (non-deleted) rows
-- are indexed; soft-deleting a row removes it from search.
CREATE TRIGGER snippets_ai AFTER INSERT ON snippets
WHEN new.deleted_at IS NULL BEGIN
  INSERT INTO snippets_fts(rowid, name, body, trigger)
  VALUES (new.rowid, new.name, new.body, new.trigger);
END;

CREATE TRIGGER snippets_ad AFTER DELETE ON snippets BEGIN
  INSERT INTO snippets_fts(snippets_fts, rowid, name, body, trigger)
  VALUES ('delete', old.rowid, old.name, old.body, old.trigger);
END;

CREATE TRIGGER snippets_au AFTER UPDATE ON snippets BEGIN
  INSERT INTO snippets_fts(snippets_fts, rowid, name, body, trigger)
  VALUES ('delete', old.rowid, old.name, old.body, old.trigger);
  INSERT INTO snippets_fts(rowid, name, body, trigger)
  SELECT new.rowid, new.name, new.body, new.trigger
  WHERE new.deleted_at IS NULL;
END;

-- FTS for clipboard text entries.
CREATE VIRTUAL TABLE clips_fts USING fts5(
  content,
  content='clip_entries',
  content_rowid='rowid',
  tokenize='unicode61 remove_diacritics 2'
);

CREATE TRIGGER clips_ai AFTER INSERT ON clip_entries
WHEN new.deleted_at IS NULL AND new.content IS NOT NULL BEGIN
  INSERT INTO clips_fts(rowid, content) VALUES (new.rowid, new.content);
END;

CREATE TRIGGER clips_ad AFTER DELETE ON clip_entries BEGIN
  INSERT INTO clips_fts(clips_fts, rowid, content) VALUES ('delete', old.rowid, old.content);
END;

CREATE TRIGGER clips_au AFTER UPDATE ON clip_entries BEGIN
  INSERT INTO clips_fts(clips_fts, rowid, content) VALUES ('delete', old.rowid, old.content);
  INSERT INTO clips_fts(rowid, content)
  SELECT new.rowid, new.content
  WHERE new.deleted_at IS NULL AND new.content IS NOT NULL;
END;
