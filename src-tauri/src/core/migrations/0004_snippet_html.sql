-- Rich-text (HTML) snippet bodies.
--
-- A snippet's `body` is always the plain-text form (what gets typed, what the
-- expansion matcher and search index see). `body_html` is an OPTIONAL rich form:
-- when present, insertion places both HTML (CF_HTML) and the plain body on the
-- clipboard, so rich targets (Outlook, Word, Gmail) get formatting while plain
-- targets degrade to the text. NULL means "plain snippet" — the common case.
--
-- We do not index body_html for search: `body` already carries the words, and
-- markup would only pollute the FTS tokens.

ALTER TABLE snippets
  ADD COLUMN body_html TEXT;
