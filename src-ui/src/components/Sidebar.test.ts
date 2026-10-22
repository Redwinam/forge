// @vitest-environment jsdom
import { describe, expect, it, vi } from 'vitest';
import { flushPromises, mount } from '@vue/test-utils';

// A Windows backend: every path uses backslashes
const tree: Record<string, { name: string; path: string; is_dir: boolean }[]> = {
  'C:\\Users\\me\\笔记': [
    { name: 'sub', path: 'C:\\Users\\me\\笔记\\sub', is_dir: true },
    { name: 'a.md', path: 'C:\\Users\\me\\笔记\\a.md', is_dir: false },
  ],
  'C:\\Users\\me\\笔记\\sub': [{ name: 'b.md', path: 'C:\\Users\\me\\笔记\\sub\\b.md', is_dir: false }],
};

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(async (cmd: string, args: any) => {
    if (cmd === 'list_files') return tree[args.path] ?? [];
    return null;
  }),
}));

import Sidebar from './Sidebar.vue';

describe('Sidebar with Windows paths', () => {
  it('shows the folder name and expands the folder holding the open file', async () => {
    const wrapper = mount(Sidebar, {
      props: { rootPath: 'C:\\Users\\me\\笔记', currentFile: 'C:\\Users\\me\\笔记\\sub\\b.md', initialWidth: null },
    });
    await flushPromises();

    expect(wrapper.find('span[title]').text()).toBe('笔记');
    const names = wrapper.find('.custom-scrollbar').findAll('span.truncate').map((s) => s.text());
    expect(names).toEqual(['sub', 'b.md', 'a.md']);
  });
});
