-- Rich clipboard content types.
--
-- Text clips are all stored the same way, but a URL, an email address, a hex
-- color, or a filesystem path each afford a type-specific action (open, reveal,
-- pick). We classify each text clip once at capture and store the result so the
-- UI can show a badge and the right action without re-scanning on every render.
--
--   content_type: 'text' | 'url' | 'email' | 'color' | 'path' | 'files' | 'image'
--
-- 'files' is a text clip whose content is one-or-more newline-separated file
-- paths captured from an Explorer file copy (CF_HDROP). 'image' mirrors the
-- existing image `kind` so a single column fully describes how to render a row.
--
-- source_app already exists (0001) but was never selected; no schema change is
-- needed for it — this migration only adds content_type and backfills it.

ALTER TABLE clip_entries
  ADD COLUMN content_type TEXT NOT NULL DEFAULT 'text';

-- Backfill existing rows: images become 'image', everything else stays 'text'.
-- We deliberately do NOT retro-classify old text as url/email/etc. — the
-- classifier runs going forward at capture time; a coarse backfill keeps this
-- migration cheap and deterministic, and old rows age out via retention.
UPDATE clip_entries SET content_type = 'image' WHERE kind = 'image';
