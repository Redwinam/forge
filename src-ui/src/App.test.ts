// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils';

const files: Record<string, string> = {};
const writes: { path: string; content: string }[] = [];
let failWrites = false;

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(async (cmd: string, args: any) => {
    switch (cmd) {
      case 'list_files':
        return Object.keys(files)
          .filter((p) => p.startsWith(args.path + '/'))
          .map((p) => ({ name: p.split('/').pop(), path: p, is_dir: false }));
      case 'read_file':
        if (!(args.path in files)) throw 'No such file or directory (os error 2)';
        return files[args.path];
      case 'write_file':
        if (failWrites) throw 'Permission denied (os error 13)';
        files[args.path] = args.content;
        writes.push({ path: args.path, content: args.content });
        return null;
      default:
        throw `unmocked command ${cmd}`;
    }
  }),
}));

vi.mock('@tauri-apps/plugin-store', () => ({
  LazyStore: class {
    async get() {
      return null;
    }
    async set() {}
    async save() {}
  },
}));

import App from './App.vue';

// The task list makes the editor's markdown differ from the file (it appends a blank line)
const NOTE_A = '---\ntitle: 旧标题\ndate: 2024-01-01\ntags: [x, y]\ncount: 3\n---\n# A\n\n正文 A\n\n- [ ] 待办\n';
const NOTE_B = '# B\n\n正文 B\n';

let wrapper: VueWrapper | null = null;

const openFile = async (name: string) => {
  const item = wrapper!.findAll('span.truncate').find((s) => s.text() === name);
  if (!item) throw new Error(`no tree item ${name}`);
  await item.trigger('click');
  await flushPromises();
  // Tiptap fires onCreate on the next tick
  await new Promise((resolve) => setTimeout(resolve, 0));
};

const metadataInput = (value: string) => {
  const input = wrapper!.findAll('input').find((i) => (i.element as HTMLInputElement).value === value);
  if (!input) throw new Error(`no metadata input with value ${value}`);
  return input;
};

const hasUnsavedMarker = () => wrapper!.find('button[title^="Save"] span.bg-red-500').exists();

beforeEach(async () => {
  for (const key of Object.keys(files)) delete files[key];
  files['/notes/a.md'] = NOTE_A;
  files['/notes/b.md'] = NOTE_B;
  writes.length = 0;
  failWrites = false;
  localStorage.setItem('forge_root_path', '/notes');
  vi.stubGlobal('alert', vi.fn());
  wrapper = mount(App, { attachTo: document.body });
  await flushPromises();
  await openFile('a.md');
});

afterEach(() => {
  wrapper?.unmount();
  wrapper = null;
  vi.unstubAllGlobals();
});

describe('editing frontmatter in the metadata panel', () => {
  it('marks the note as unsaved', async () => {
    expect(hasUnsavedMarker()).toBe(false);
    await metadataInput('旧标题').setValue('新标题');
    expect(hasUnsavedMarker()).toBe(true);
  });

  it('is saved when switching to another file', async () => {
    await metadataInput('旧标题').setValue('新标题');
    await openFile('b.md');

    expect(writes).toHaveLength(1);
    expect(writes[0]!.path).toBe('/notes/a.md');
    expect(writes[0]!.content.startsWith('---\ntitle: 新标题\ndate: 2024-01-01\ntags: [x, y]\ncount: 3\n---\n')).toBe(
      true,
    );
    expect(wrapper!.find('.ProseMirror').text()).toContain('正文 B');
  });

  it('keeps the note open when that save fails', async () => {
    failWrites = true;
    await metadataInput('旧标题').setValue('新标题');
    await openFile('b.md');

    expect(wrapper!.find('.ProseMirror').text()).toContain('正文 A');
    expect(metadataInput('新标题').exists()).toBe(true);
    expect(hasUnsavedMarker()).toBe(true);
  });

  it('is clean again after the edit is reverted', async () => {
    await metadataInput('旧标题').setValue('新标题');
    await metadataInput('新标题').setValue('旧标题');
    expect(hasUnsavedMarker()).toBe(false);
    await openFile('b.md');
    expect(writes).toHaveLength(0);
  });

  it('writes numeric fields back as numbers', async () => {
    await metadataInput('3').setValue('4');
    await openFile('b.md');
    expect(writes[0]!.content).toContain('\ncount: 4\n');
  });
});

describe('switching files', () => {
  it('does not write a note that was only opened', async () => {
    await openFile('b.md');
    expect(writes).toHaveLength(0);
    expect(files['/notes/a.md']).toBe(NOTE_A);
  });
});
