// Split a string into matched / unmatched segments for a search query, so the
// palette can wrap the matched part in <mark> and show *why* a row matched.
//
// Matching is case-insensitive and matches every occurrence of the (trimmed)
// query as a literal substring — the same "contains" feel as the FTS-backed
// list, without pulling the query's ranking logic into the UI. Returns a flat
// list of {text, match} chunks in original order; callers render `match` chunks
// as <mark> and the rest as plain text.

export interface HlChunk {
  text: string;
  match: boolean;
}

export function highlight(source: string, query: string): HlChunk[] {
  const q = query.trim();
  if (!q || !source) return [{ text: source, match: false }];

  const hay = source.toLowerCase();
  const needle = q.toLowerCase();
  const chunks: HlChunk[] = [];
  let i = 0;

  while (i < source.length) {
    const at = hay.indexOf(needle, i);
    if (at === -1) {
      chunks.push({ text: source.slice(i), match: false });
      break;
    }
    if (at > i) chunks.push({ text: source.slice(i, at), match: false });
    chunks.push({ text: source.slice(at, at + needle.length), match: true });
    i = at + needle.length;
  }

  return chunks;
}
