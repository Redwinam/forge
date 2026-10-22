import type { Editor } from '@tiptap/core';

const ESCAPABLE = /[\\`*_[\]~]/;
// The serializer turns every `&` into `&amp;`. Loading decodes only these references, so only
// these must stay encoded; anything else (`&nbsp;`, `&copy;`, a bare `AT&T`) is written as is.
const DECODED_ON_LOAD = /&amp;(?!(?:#\d{1,7}|#[xX][0-9a-fA-F]{1,6}|lt|gt|quot|amp);)/g;
const WORD_CHAR = /[\p{L}\p{N}]/u;
// `[[...]]` stays plain text in CommonMark unless a link destination follows it
const DOUBLE_BRACKETS = /\[\[[^[\]\n]+\]\](?!\()/g;

/**
 * Escapes plain text for markdown like Tiptap 3.31 does, minus what CommonMark already reads
 * literally and the escapes would change or clutter:
 * - `[[wiki links]]` / VitePress `[[toc]]`;
 * - underscores inside words (snake_case, file_name.md), which can never start emphasis;
 * - `&` that doesn't start a reference the loader decodes, so `&nbsp;` keeps meaning a space.
 * `text` arrives HTML-encoded (`&`, `<`, `>` already turned into entities).
 */
export function escapeMarkdownText(encoded: string): string {
  const text = encoded.replace(DECODED_ON_LOAD, '&');
  const literal = new Set<number>();
  for (const match of text.matchAll(DOUBLE_BRACKETS)) {
    const start = match.index;
    const end = start + match[0].length;
    for (const i of [start, start + 1, end - 2, end - 1]) literal.add(i);
  }

  let out = '';
  for (let i = 0; i < text.length; i++) {
    const ch = text[i]!;
    const intraword = ch === '_' && WORD_CHAR.test(text[i - 1] ?? '') && WORD_CHAR.test(text[i + 1] ?? '');
    out += ESCAPABLE.test(ch) && !literal.has(i) && !intraword ? `\\${ch}` : ch;
  }
  return out;
}

/**
 * Swaps in escapeMarkdownText for the serializer's own escaping. That method is internal to
 * @tiptap/markdown, so this fails loudly if it disappears; the markdown snapshot tests cover it.
 */
export function useLenientEscaping(editor: Editor) {
  const manager = editor.markdown as unknown as { escapeMarkdownSyntax?: (text: string) => string } | undefined;
  if (typeof manager?.escapeMarkdownSyntax !== 'function') {
    throw new Error('@tiptap/markdown no longer has escapeMarkdownSyntax; revisit markdown-escape.ts');
  }
  manager.escapeMarkdownSyntax = escapeMarkdownText;
}
