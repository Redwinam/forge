import { describe, expect, it } from 'vitest';
import { escapeMarkdownText } from './markdown-escape';

describe('escapeMarkdownText', () => {
  it('escapes characters that could turn into formatting', () => {
    expect(escapeMarkdownText('literal *star* and ~tilde~')).toBe('literal \\*star\\* and \\~tilde\\~');
    expect(escapeMarkdownText('a `tick` and C:\\Users')).toBe('a \\`tick\\` and C:\\\\Users');
    expect(escapeMarkdownText('_emphasis_ and __strong__')).toBe('\\_emphasis\\_ and \\_\\_strong\\_\\_');
    expect(escapeMarkdownText('text [not a link](really)')).toBe('text \\[not a link\\](really)');
  });

  it('leaves wiki links and the VitePress table of contents alone', () => {
    expect(escapeMarkdownText('see [[双链笔记]] and [[toc]]')).toBe('see [[双链笔记]] and [[toc]]');
    expect(escapeMarkdownText('[[Page|alias]]')).toBe('[[Page|alias]]');
  });

  it('still escapes double brackets that a link destination follows', () => {
    expect(escapeMarkdownText('[[x]](url)')).toBe('\\[\\[x\\]\\](url)');
    expect(escapeMarkdownText('[[a] b]')).toBe('\\[\\[a\\] b\\]');
  });

  it('leaves underscores inside words alone', () => {
    expect(escapeMarkdownText('snake_case_words and file_name.md')).toBe('snake_case_words and file_name.md');
    expect(escapeMarkdownText('中文_下划线_中文')).toBe('中文_下划线_中文');
    expect(escapeMarkdownText('trailing_ _leading')).toBe('trailing\\_ \\_leading');
  });

  it('keeps entities and ampersands as written', () => {
    // Input is what the serializer passes in, already HTML-encoded
    expect(escapeMarkdownText('a&amp;nbsp;b &amp;copy; AT&amp;T')).toBe('a&nbsp;b &copy; AT&T');
    expect(escapeMarkdownText('a &lt; b &gt; c')).toBe('a &lt; b &gt; c');
    // Literal "&lt;" or "&#169;" text must stay encoded or it would load back as < and ©
    expect(escapeMarkdownText('&amp;lt; &amp;#169; &amp;#x41; &amp;amp;')).toBe('&amp;lt; &amp;#169; &amp;#x41; &amp;amp;');
  });
});
