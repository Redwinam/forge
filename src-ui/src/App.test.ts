// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils';

const files: Record<string, string> = {};
const writes: { path: string; content: string }[] = [];
const calls: { cmd: string; args: any }[] = [];
let failWrites = false;
let rootDir: string | null = '/notes';
let pickedRoot: string | null = null;

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(async (cmd: string, args: any) => {
    calls.push({ cmd, args });
    switch (cmd) {
      case 'load_app_state':
        return { rootDir, lastOpenedFile: null, sidebarWidth: null };
      case 'choose_root_dir':
        rootDir = pickedRoot ?? rootDir;
        return pickedRoot;
      case 'set_last_opened_file':
      case 'set_sidebar_width':
        return null;
      case 'get_cos_settings':
        return { bucket: 'b-1', region: 'ap-shanghai', cdnDomain: '', prefix: 'press/', hasCredentials: true };
      case 'save_cos_settings':
      case 'clear_cos_credentials':
        return null;
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
  calls.length = 0;
  failWrites = false;
  rootDir = '/notes';
  pickedRoot = null;
  localStorage.clear();
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

describe('notes folder', () => {
  it('is changed through the native picker after saving the open note', async () => {
    files['/other/c.md'] = '# C\n';
    pickedRoot = '/other';
    await metadataInput('旧标题').setValue('新标题');

    await wrapper!.find('button[title="Settings"]').trigger('click');
    const choose = wrapper!.findAll('button').find((b) => b.text() === 'Choose…')!;
    await choose.trigger('click');
    await flushPromises();

    expect(writes.map((w) => w.path)).toEqual(['/notes/a.md']);
    expect(calls.some((c) => c.cmd === 'choose_root_dir')).toBe(true);
    // The open note is outside the new folder, so it is closed
    expect(wrapper!.find('.ProseMirror').exists()).toBe(false);
    expect(wrapper!.text()).toContain('c.md');
  });

  it('hands the root kept by older versions to the backend once', async () => {
    wrapper!.unmount();
    rootDir = null;
    localStorage.setItem('forge_root_path', '/legacy');
    calls.length = 0;
    wrapper = mount(App, { attachTo: document.body });
    await flushPromises();

    expect(calls.find((c) => c.cmd === 'load_app_state')!.args).toEqual({ legacyRootDir: '/legacy' });
  });
});

describe('COS settings', () => {
  const openSettings = async () => {
    await wrapper!.find('button[title="Settings"]').trigger('click');
    await flushPromises();
  };
  const keyInputs = () => wrapper!.findAll('input[type="password"]');

  it('never shows the saved keys and keeps them when left empty', async () => {
    await openSettings();
    expect(keyInputs().map((i) => (i.element as HTMLInputElement).value)).toEqual(['', '']);
    expect(wrapper!.text()).toContain('stored in the system keychain');

    await wrapper!.findAll('button').find((b) => b.text() === 'Save Changes')!.trigger('click');
    await flushPromises();
    const save = calls.find((c) => c.cmd === 'save_cos_settings')!;
    expect(save.args.settings).toMatchObject({ secretId: '', secretKey: '', bucket: 'b-1', region: 'ap-shanghai' });
  });

  it('sends newly typed keys once', async () => {
    await openSettings();
    await keyInputs()[0]!.setValue('AKIDnew');
    await keyInputs()[1]!.setValue('new-key');
    await wrapper!.findAll('button').find((b) => b.text() === 'Save Changes')!.trigger('click');
    await flushPromises();
    expect(calls.find((c) => c.cmd === 'save_cos_settings')!.args.settings).toMatchObject({
      secretId: 'AKIDnew',
      secretKey: 'new-key',
    });
  });
});
