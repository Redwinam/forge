// Paths come from the backend in the platform's own form, so Windows paths use backslashes
// (C:\notes\a.md). These helpers treat both separators alike.

const WINDOWS_PATH = /^([A-Za-z]:|[\\/]{2})/;

/** Forward slashes only, without a trailing separator unless the path is a root. */
export function normalizePath(path: string): string {
  const normalized = path.replace(/\\/g, '/');
  const trimmed = normalized.replace(/\/+$/, '');
  if (trimmed === '') return '/';
  if (/^[A-Za-z]:$/.test(trimmed)) return `${trimmed}/`;
  return trimmed;
}

export function basename(path: string): string {
  const normalized = normalizePath(path);
  if (normalized === '/' || /^[A-Za-z]:\/$/.test(normalized)) return normalized;
  return normalized.slice(normalized.lastIndexOf('/') + 1);
}

/** True when `path` is somewhere below `dir` (not `dir` itself). */
export function isInside(path: string, dir: string): boolean {
  let child = normalizePath(path);
  let parent = normalizePath(dir);
  // NTFS is case-insensitive; POSIX filesystems usually aren't
  if (WINDOWS_PATH.test(child) && WINDOWS_PATH.test(parent)) {
    child = child.toLowerCase();
    parent = parent.toLowerCase();
  }
  const prefix = parent.endsWith('/') ? parent : `${parent}/`;
  return child.startsWith(prefix) && child.length > prefix.length;
}
