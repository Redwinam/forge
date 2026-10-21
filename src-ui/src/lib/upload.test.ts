import { beforeEach, describe, expect, it, vi } from 'vitest';

const invoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({ invoke: (...args: unknown[]) => invoke(...args) }));

import { MAX_FILE_BYTES, MAX_IMAGE_BYTES, uploadExtension, uploadToCos } from './upload';

const file = (name: string, type: string, bytes: number[] = [1, 2, 3]) =>
  new File([new Uint8Array(bytes)], name, { type });

describe('uploadExtension', () => {
  it('takes the extension from the file name', () => {
    expect(uploadExtension(file('photo.JPG', 'image/jpeg'), 'image')).toBe('jpg');
    expect(uploadExtension(file('archive.tar.gz', 'application/gzip'), 'file')).toBe('gz');
  });

  it('falls back to the MIME type when the name has no extension', () => {
    expect(uploadExtension(file('image', 'image/png'), 'image')).toBe('png');
    expect(uploadExtension(file('.hidden', 'image/svg+xml'), 'image')).toBe('svg');
  });

  it('uses bin for other files without an extension', () => {
    expect(uploadExtension(file('Makefile', ''), 'file')).toBe('bin');
  });

  it('does not treat a whole file name as an extension', () => {
    expect(uploadExtension(file('README', 'text/plain'), 'file')).toBe('bin');
  });

  it('ignores an extension that would not be a valid key segment', () => {
    expect(() => uploadExtension(file('x.p ng', 'image/png'), 'image')).not.toThrow();
    expect(uploadExtension(file('x.p ng', 'image/png'), 'image')).toBe('png');
    expect(uploadExtension(file('报告.文档', ''), 'file')).toBe('bin');
  });

  it('rejects files that are not images when an image is expected', () => {
    expect(() => uploadExtension(file('script.exe', 'application/octet-stream'), 'image')).toThrow(/image/i);
  });
});

describe('uploadToCos', () => {
  beforeEach(() => {
    invoke.mockReset();
    invoke.mockResolvedValue('https://cdn.example.com/press/images/x.png');
  });

  it('sends the bytes as a raw body with the extension in a header', async () => {
    const url = await uploadToCos(file('a.png', 'image/png', [9, 8, 7]), 'image');

    expect(url).toBe('https://cdn.example.com/press/images/x.png');
    const [cmd, body, options] = invoke.mock.calls[0]!;
    expect(cmd).toBe('upload_image');
    expect(body).toBeInstanceOf(Uint8Array);
    expect(Array.from(body as Uint8Array)).toEqual([9, 8, 7]);
    expect(options).toEqual({ headers: { 'X-Forge-Extension': 'png' } });
  });

  it('uses the file command for attachments', async () => {
    await uploadToCos(file('doc.pdf', 'application/pdf'), 'file');
    expect(invoke.mock.calls[0]![0]).toBe('upload_file');
  });

  it('refuses empty and oversized files before reading them', async () => {
    await expect(uploadToCos(file('a.png', 'image/png', []), 'image')).rejects.toThrow(/empty/);

    const big = (size: number, name: string, type: string) => {
      const f = file(name, type);
      Object.defineProperty(f, 'size', { value: size });
      return f;
    };
    await expect(uploadToCos(big(MAX_IMAGE_BYTES + 1, 'a.png', 'image/png'), 'image')).rejects.toThrow(/too large/);
    await expect(uploadToCos(big(MAX_FILE_BYTES + 1, 'a.zip', 'application/zip'), 'file')).rejects.toThrow(/too large/);
    expect(invoke).not.toHaveBeenCalled();
  });
});
