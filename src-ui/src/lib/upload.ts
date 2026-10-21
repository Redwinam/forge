import { invoke } from '@tauri-apps/api/core';

export type UploadKind = 'image' | 'file';

// Keep in sync with src-tauri/src/cos.rs, which enforces the same rules
export const MAX_IMAGE_BYTES = 20 * 1024 * 1024;
export const MAX_FILE_BYTES = 100 * 1024 * 1024;

const IMAGE_EXTENSIONS = ['png', 'jpg', 'jpeg', 'gif', 'webp', 'avif', 'bmp', 'svg', 'ico', 'tif', 'tiff', 'heic', 'heif'];

const MIME_EXTENSIONS: Record<string, string> = {
  'image/png': 'png',
  'image/jpeg': 'jpg',
  'image/gif': 'gif',
  'image/webp': 'webp',
  'image/avif': 'avif',
  'image/bmp': 'bmp',
  'image/svg+xml': 'svg',
  'image/x-icon': 'ico',
  'image/tiff': 'tiff',
  'image/heic': 'heic',
  'image/heif': 'heif',
};

/** The extension used in the object key: from the file name, else from the MIME type. */
export function uploadExtension(file: File, kind: UploadKind): string {
  const fromName = /[^.]\.([A-Za-z0-9]{1,10})$/.exec(file.name)?.[1]?.toLowerCase();
  const extension = fromName ?? MIME_EXTENSIONS[file.type] ?? (kind === 'file' ? 'bin' : undefined);
  if (kind === 'image' && (!extension || !IMAGE_EXTENSIONS.includes(extension))) {
    throw new Error(`Not a supported image type: ${file.name || file.type}`);
  }
  return extension!;
}

function checkSize(file: File, kind: UploadKind) {
  const limit = kind === 'image' ? MAX_IMAGE_BYTES : MAX_FILE_BYTES;
  if (file.size === 0) throw new Error('The file is empty');
  if (file.size > limit) {
    throw new Error(`The file is too large (${(file.size / 1024 / 1024).toFixed(1)} MB, limit ${limit / 1024 / 1024} MB)`);
  }
}

/** Uploads to Tencent COS and returns the public URL. */
export async function uploadToCos(file: File, kind: UploadKind): Promise<string> {
  checkSize(file, kind);
  const extension = uploadExtension(file, kind);
  // A Uint8Array goes over IPC as a raw body; a plain array would be JSON-encoded number by number
  const bytes = new Uint8Array(await file.arrayBuffer());
  return invoke<string>(kind === 'image' ? 'upload_image' : 'upload_file', bytes, {
    headers: { 'X-Forge-Extension': extension },
  });
}
