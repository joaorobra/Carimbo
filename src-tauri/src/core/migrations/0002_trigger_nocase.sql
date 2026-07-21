-- Make snippet triggers case-insensitive for uniqueness.
--
-- A user who types ";cpf" doesn't distinguish it from ";CPF", so two live
-- snippets differing only in trigger case are a footgun, not a feature. The
-- original UNIQUE index used the default BINARY collation and allowed both.
--
-- This migration:
--   1. Resolves any existing case-collisions among live triggers by soft-
--      deleting all but the most-recently-updated snippet in each collision
--      group (keeps the newest, tombstones older ones and frees their trigger).
--   2. Replaces the index with a COLLATE NOCASE variant so future inserts of a
--      case-variant trigger are rejected as duplicates.
--
-- NOCASE only case-folds ASCII A–Z, which covers the ASCII-punctuation +
-- letter triggers Carimbo uses (";cpf", ";end"). Non-ASCII trigger characters
-- remain compared as-is, which is acceptable.

-- 1. Tombstone older members of each case-insensitive collision group.
-- The survivor is the row with the greatest updated_at (ties broken by id).
UPDATE snippets
SET deleted_at = CAST(strftime('%s','now') AS INTEGER) * 1000,
    trigger    = NULL,
    updated_at = CAST(strftime('%s','now') AS INTEGER) * 1000
WHERE deleted_at IS NULL
  AND trigger IS NOT NULL
  AND id NOT IN (
    SELECT id FROM (
      SELECT id,
             ROW_NUMBER() OVER (
               PARTITION BY lower(trigger)
               ORDER BY updated_at DESC, id DESC
             ) AS rn
      FROM snippets
      WHERE deleted_at IS NULL AND trigger IS NOT NULL
    )
    WHERE rn = 1
  );

-- 2. Swap the BINARY-collation unique index for a NOCASE one.
DROP INDEX IF EXISTS idx_snippets_trigger;
CREATE UNIQUE INDEX idx_snippets_trigger
  ON snippets(trigger COLLATE NOCASE)
  WHERE deleted_at IS NULL AND trigger IS NOT NULL;
