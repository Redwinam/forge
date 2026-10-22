// @vitest-environment jsdom
// Regression net for the markdown the editor writes back to disk. The snapshots in
// __snapshots__/ are what the app saves for each fixture after opening it unchanged;
// review any diff there before accepting it, especially after upgrading Tiptap.
import { describe, expect, it } from 'vitest';
import { Editor } from '@tiptap/core';
import { editorExtensions } from './extensions';
import { joinFrontmatter, serializeFrontmatter, splitFrontmatter } from '../../lib/frontmatter';

const fixtures = import.meta.glob<string>('./__fixtures__/*.md', { query: '?raw', import: 'default', eager: true });
const cases = Object.entries(fixtures).map(([path, source]) => ({ name: path.split('/').pop()!, source }));

// Same path as opening a note in Editor.vue and saving it without edits
function save(full: string): string {
  const { frontmatter, body } = splitFrontmatter(full);
  const editor = new Editor({ extensions: editorExtensions(), content: body, contentType: 'markdown' });
  try {
    return joinFrontmatter(serializeFrontmatter(frontmatter, frontmatter.data), editor.getMarkdown());
  } finally {
    editor.destroy();
  }
}

describe.each(cases)('$name', ({ name, source }) => {
  it('saves the expected markdown', async () => {
    await expect(save(source)).toMatchFileSnapshot(`./__snapshots__/${name}`);
  });

  it('is stable when saved again', () => {
    const once = save(source);
    expect(save(once)).toBe(once);
  });

  it('keeps the frontmatter byte for byte', () => {
    expect(splitFrontmatter(save(source)).frontmatter.block).toBe(splitFrontmatter(source).frontmatter.block);
  });
});
