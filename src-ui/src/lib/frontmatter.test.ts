import { describe, expect, it } from 'vitest';
import { parse } from 'yaml';
import { joinFrontmatter, serializeFrontmatter, splitFrontmatter } from './frontmatter';

const RICH = [
  '---',
  'title: 冒烟测试笔记',
  'date: 2024-01-01',
  'tags: [smoke, yaml]',
  '# 注释应保留',
  '',
  'draft: false',
  'link: "https://example.com/a?b=c"',
  "quote: 'single'",
  'count: 3',
  'authors:',
  '- alice',
  '- bob',
  '---',
  '',
].join('\n');

const clone = <T>(v: T): T => structuredClone(v);

describe('splitFrontmatter', () => {
  it('returns the whole text as body when there is no frontmatter', () => {
    const { frontmatter, body } = splitFrontmatter('# Title\n\nbody\n');
    expect(frontmatter.block).toBe('');
    expect(frontmatter.data).toEqual({});
    expect(body).toBe('# Title\n\nbody\n');
  });

  it('keeps dates as plain strings instead of Date objects', () => {
    const { frontmatter, body } = splitFrontmatter(RICH + '# Body\n');
    expect(frontmatter.editable).toBe(true);
    expect(frontmatter.data.date).toBe('2024-01-01');
    expect(frontmatter.data.tags).toEqual(['smoke', 'yaml']);
    expect(frontmatter.data.count).toBe(3);
    expect(body).toBe('# Body\n');
  });

  it('marks invalid YAML as not editable and keeps the block', () => {
    const text = '---\ntitle: [unclosed\n---\nbody\n';
    const { frontmatter, body } = splitFrontmatter(text);
    expect(frontmatter.editable).toBe(false);
    expect(frontmatter.data).toEqual({});
    expect(frontmatter.block).toBe('---\ntitle: [unclosed\n---\n');
    expect(body).toBe('body\n');
  });

  it('marks a non-mapping frontmatter as not editable', () => {
    const { frontmatter } = splitFrontmatter('---\n- a\n- b\n---\nbody\n');
    expect(frontmatter.editable).toBe(false);
  });

  it('recognises an empty frontmatter block', () => {
    const { frontmatter, body } = splitFrontmatter('---\n---\nbody\n');
    expect(frontmatter.block).toBe('---\n---\n');
    expect(body).toBe('body\n');
  });

  it('recognises frontmatter at the end of the file without a trailing newline', () => {
    const { frontmatter, body } = splitFrontmatter('---\ntitle: a\n---');
    expect(frontmatter.data).toEqual({ title: 'a' });
    expect(body).toBe('');
  });
});

describe('serializeFrontmatter', () => {
  it('returns the original block byte-for-byte when nothing changed', () => {
    const { frontmatter } = splitFrontmatter(RICH + 'body\n');
    expect(serializeFrontmatter(frontmatter, clone(frontmatter.data))).toBe(RICH);
  });

  it('keeps CRLF line endings when nothing changed', () => {
    const crlf = RICH.replace(/\n/g, '\r\n');
    const { frontmatter, body } = splitFrontmatter(crlf + 'body\r\n');
    expect(body).toBe('body\r\n');
    expect(serializeFrontmatter(frontmatter, clone(frontmatter.data))).toBe(crlf);
  });

  it('only rewrites the key that changed', () => {
    const { frontmatter } = splitFrontmatter(RICH);
    const data = clone(frontmatter.data);
    data.title = '新标题';
    const out = serializeFrontmatter(frontmatter, data);
    expect(out).toBe(RICH.replace('title: 冒烟测试笔记', 'title: 新标题'));
  });

  it('keeps the quoting style of an edited value', () => {
    const { frontmatter } = splitFrontmatter(RICH);
    const data = clone(frontmatter.data);
    data.link = 'https://example.com/other';
    data.quote = 'changed';
    const out = serializeFrontmatter(frontmatter, data);
    expect(out).toBe(
      RICH.replace('link: "https://example.com/a?b=c"', 'link: "https://example.com/other"').replace(
        "quote: 'single'",
        "quote: 'changed'",
      ),
    );
  });

  it('quotes a new value when it would not survive as a plain scalar', () => {
    const { frontmatter } = splitFrontmatter(RICH);
    const data = clone(frontmatter.data);
    data.title = 'a: b # not a comment';
    const out = serializeFrontmatter(frontmatter, data);
    expect(parse(out.replace(/^---\n|---\n$/g, '')).title).toBe('a: b # not a comment');
    expect(out.split('\n').filter((l) => !l.startsWith('title:'))).toEqual(
      RICH.split('\n').filter((l) => !l.startsWith('title:')),
    );
  });

  it('writes numbers back as numbers', () => {
    const { frontmatter } = splitFrontmatter(RICH);
    const data = clone(frontmatter.data);
    data.count = 4;
    expect(serializeFrontmatter(frontmatter, data)).toBe(RICH.replace('count: 3', 'count: 4'));
  });

  it('keeps CRLF line endings around an edited value', () => {
    const crlf = RICH.replace(/\n/g, '\r\n');
    const { frontmatter } = splitFrontmatter(crlf);
    const data = clone(frontmatter.data);
    data.title = 'x';
    expect(serializeFrontmatter(frontmatter, data)).toBe(crlf.replace('title: 冒烟测试笔记', 'title: x'));
  });

  it('fills in a key that had no value', () => {
    const text = '---\ntitle:\ntags: [a, b]\n---\n';
    const { frontmatter } = splitFrontmatter(text);
    const data = clone(frontmatter.data);
    data.title = 'filled';
    const out = serializeFrontmatter(frontmatter, data);
    expect(out).toBe('---\ntitle: filled\ntags: [a, b]\n---\n');
  });

  it('keeps flow sequences and comments when a key is removed', () => {
    const { frontmatter } = splitFrontmatter(RICH);
    const data = clone(frontmatter.data);
    delete data.count;
    const out = serializeFrontmatter(frontmatter, data);
    expect(out).toContain('tags: [smoke, yaml]');
    expect(out).toContain('# 注释应保留');
    expect(out).toContain('date: 2024-01-01\n');
    expect(out).toContain('link: "https://example.com/a?b=c"');
    expect(out).not.toContain('count');
    expect(parse(out.replace(/^---\n|---\n$/g, ''))).toEqual(data);
  });

  it('never rewrites a block it could not parse', () => {
    const { frontmatter } = splitFrontmatter('---\ntitle: [unclosed\n---\nbody\n');
    expect(serializeFrontmatter(frontmatter, { title: 'x' })).toBe('---\ntitle: [unclosed\n---\n');
  });

  it('keeps an empty frontmatter block', () => {
    const { frontmatter } = splitFrontmatter('---\n---\nbody\n');
    expect(serializeFrontmatter(frontmatter, {})).toBe('---\n---\n');
  });
});

describe('joinFrontmatter', () => {
  it('puts the body after the block', () => {
    expect(joinFrontmatter('---\na: 1\n---\n', 'body\n')).toBe('---\na: 1\n---\nbody\n');
    expect(joinFrontmatter('', 'body\n')).toBe('body\n');
  });

  it('adds a line break when the block ended the file', () => {
    expect(joinFrontmatter('---\na: 1\n---', 'body\n')).toBe('---\na: 1\n---\nbody\n');
    expect(joinFrontmatter('---\na: 1\n---', '')).toBe('---\na: 1\n---');
  });
});
