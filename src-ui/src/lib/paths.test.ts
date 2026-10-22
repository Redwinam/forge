import { describe, expect, it } from 'vitest';
import { basename, isInside, normalizePath } from './paths';

describe('normalizePath', () => {
  it('uses forward slashes and drops trailing separators', () => {
    expect(normalizePath('C:\\Users\\me\\notes\\')).toBe('C:/Users/me/notes');
    expect(normalizePath('/home/me/notes/')).toBe('/home/me/notes');
    expect(normalizePath('\\\\server\\share\\notes')).toBe('//server/share/notes');
  });

  it('keeps filesystem roots intact', () => {
    expect(normalizePath('/')).toBe('/');
    expect(normalizePath('C:\\')).toBe('C:/');
  });
});

describe('basename', () => {
  it('works with either separator', () => {
    expect(basename('/Users/me/notes')).toBe('notes');
    expect(basename('C:\\Users\\me\\笔记')).toBe('笔记');
    expect(basename('C:\\Users\\me\\notes\\')).toBe('notes');
    expect(basename('/')).toBe('/');
  });
});

describe('isInside', () => {
  it('matches files below a folder', () => {
    expect(isInside('/notes/sub/a.md', '/notes')).toBe(true);
    expect(isInside('/notes/sub/a.md', '/notes/sub')).toBe(true);
    expect(isInside('C:\\notes\\sub\\a.md', 'C:\\notes\\sub')).toBe(true);
  });

  it('does not match the folder itself or a sibling sharing its prefix', () => {
    expect(isInside('/notes', '/notes')).toBe(false);
    expect(isInside('/notes-old/a.md', '/notes')).toBe(false);
    expect(isInside('C:\\notes-old\\a.md', 'C:\\notes')).toBe(false);
  });

  it('accepts a folder given with a trailing separator', () => {
    expect(isInside('/notes/a.md', '/notes/')).toBe(true);
    expect(isInside('C:\\notes\\a.md', 'C:\\notes\\')).toBe(true);
    expect(isInside('/a.md', '/')).toBe(true);
    expect(isInside('C:\\a.md', 'C:\\')).toBe(true);
  });

  it('ignores case on Windows drive paths only', () => {
    expect(isInside('c:\\Notes\\a.md', 'C:\\notes')).toBe(true);
    expect(isInside('/Notes/a.md', '/notes')).toBe(false);
  });
});
