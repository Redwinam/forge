import { Document, isMap, isScalar, parse, parseDocument, Scalar, stringify } from 'yaml';

export type FrontmatterData = Record<string, unknown>;

export interface Frontmatter {
  /** The whole block as it appears in the file, fences included; '' when the note has none. */
  block: string;
  /** The YAML between the fences. */
  yaml: string;
  /** Offset of `yaml` inside `block`. */
  yamlStart: number;
  /** Top-level mapping, parsed with the YAML 1.2 core schema so dates stay strings. */
  data: FrontmatterData;
  /** False when the YAML is invalid or not a mapping: the block is then kept verbatim. */
  editable: boolean;
}

// Opening fence, optional YAML, closing fence; the closing fence may end the file.
const FRONTMATTER_RE = /^(---[ \t]*\r?\n)(?:([\s\S]*?)\r?\n)?---[ \t]*(?:\r?\n|$)/;

// Keep long values on one line and flow collections as `[a, b]`.
const STRINGIFY_OPTIONS = { lineWidth: 0, flowCollectionPadding: false } as const;

const NO_FRONTMATTER: Frontmatter = { block: '', yaml: '', yamlStart: 0, data: {}, editable: true };

export function splitFrontmatter(full: string): { frontmatter: Frontmatter; body: string } {
  const match = full.match(FRONTMATTER_RE);
  if (!match) return { frontmatter: { ...NO_FRONTMATTER, data: {} }, body: full };

  const block = match[0];
  const yaml = match[2] ?? '';
  const frontmatter: Frontmatter = { block, yaml, yamlStart: (match[1] ?? '').length, data: {}, editable: false };
  const doc = parseDocument(yaml);
  if (doc.errors.length === 0 && (doc.contents === null || isMap(doc.contents))) {
    frontmatter.data = (doc.toJS() as FrontmatterData | null) ?? {};
    frontmatter.editable = true;
  }
  return { frontmatter, body: full.slice(block.length) };
}

export function joinFrontmatter(block: string, body: string): string {
  if (block && body && !block.endsWith('\n')) return `${block}\n${body}`;
  return block + body;
}

/**
 * Renders `data` as a frontmatter block, touching only the keys that differ from what was
 * parsed: unchanged notes keep their exact bytes, and an edited scalar is replaced in place
 * so comments, quoting, flow lists and dates elsewhere stay as the author wrote them.
 */
export function serializeFrontmatter(frontmatter: Frontmatter, data: FrontmatterData): string {
  if (!frontmatter.editable || deepEqual(frontmatter.data, data)) return frontmatter.block;

  if (!frontmatter.yaml.trim()) {
    if (Object.keys(data).length === 0) return frontmatter.block;
    const fence = frontmatter.block ? frontmatter.block.slice(0, frontmatter.yamlStart) : '---\n';
    return `${fence}${stringify(data, STRINGIFY_OPTIONS)}---\n`;
  }

  const eol = frontmatter.yaml.includes('\r\n') ? '\r\n' : '\n';
  const yaml = replaceScalarsInPlace(frontmatter, data) ?? rewriteChangedKeys(frontmatter, data, eol);
  const start = frontmatter.yamlStart;
  return frontmatter.block.slice(0, start) + yaml + frontmatter.block.slice(start + frontmatter.yaml.length);
}

// Splices new values into the source text; returns null when any change can't be done that way.
function replaceScalarsInPlace(frontmatter: Frontmatter, data: FrontmatterData): string | null {
  const doc = parseDocument(frontmatter.yaml);
  if (!isMap(doc.contents)) return null;

  const edits: { start: number; end: number; text: string }[] = [];
  for (const key of changedKeys(frontmatter.data, data)) {
    if (!(key in data)) return null;
    const node = doc.contents.items.find((pair) => isScalar(pair.key) && String(pair.key.value) === key)?.value;
    const value = data[key];
    if (!isScalar(node) || !node.range || node.range[0] === node.range[1] || node.tag || node.anchor) return null;
    if (!isInlineScalarType(node.type) || !isPrimitive(value)) return null;

    const scalar = new Scalar(value);
    scalar.type = node.type;
    const text = stringify(scalar, STRINGIFY_OPTIONS).replace(/\n$/, '');
    if (text.includes('\n')) return null;
    edits.push({ start: node.range[0], end: node.range[1], text });
  }

  let yaml = frontmatter.yaml;
  for (const edit of edits.sort((a, b) => b.start - a.start)) {
    yaml = yaml.slice(0, edit.start) + edit.text + yaml.slice(edit.end);
  }
  // Guard against any context where the spliced text reads differently than intended.
  return parsesTo(yaml, data) ? yaml : null;
}

// Fallback: edit the parsed document and re-render it, which still keeps comments and styles.
function rewriteChangedKeys(frontmatter: Frontmatter, data: FrontmatterData, eol: string): string {
  const doc = parseDocument(frontmatter.yaml);
  for (const key of changedKeys(frontmatter.data, data)) {
    if (!(key in data)) {
      doc.delete(key);
      continue;
    }
    const node = doc.get(key, true);
    if (isScalar(node) && isPrimitive(data[key])) node.value = data[key];
    else doc.set(key, doc.createNode(data[key]));
  }

  let yaml = doc.toString(STRINGIFY_OPTIONS);
  if (!parsesTo(yaml, data)) yaml = new Document(data).toString(STRINGIFY_OPTIONS);
  yaml = yaml.replace(/\n$/, '');
  return eol === '\n' ? yaml : yaml.replace(/\r?\n/g, eol);
}

function parsesTo(yaml: string, data: FrontmatterData): boolean {
  try {
    return deepEqual(parse(yaml) ?? {}, data);
  } catch {
    return false;
  }
}

function changedKeys(before: FrontmatterData, after: FrontmatterData): string[] {
  const keys = new Set([...Object.keys(before), ...Object.keys(after)]);
  return [...keys].filter((key) => !(key in before && key in after && deepEqual(before[key], after[key])));
}

function isInlineScalarType(type: Scalar['type']): boolean {
  return type === Scalar.PLAIN || type === Scalar.QUOTE_DOUBLE || type === Scalar.QUOTE_SINGLE;
}

function isPrimitive(value: unknown): value is string | number | boolean | null {
  return value === null || ['string', 'number', 'boolean'].includes(typeof value);
}

/** Text typed into a field that held a number stays a number when it still reads as one. */
export function coerceMetadataInput(previous: unknown, input: string): string | number {
  if (typeof previous === 'number' && input.trim() !== '' && Number.isFinite(Number(input))) return Number(input);
  return input;
}

export function deepEqual(a: unknown, b: unknown): boolean {
  if (Object.is(a, b)) return true;
  if (typeof a !== 'object' || typeof b !== 'object' || a === null || b === null) return false;
  if (Array.isArray(a) !== Array.isArray(b)) return false;
  const aKeys = Object.keys(a);
  const bKeys = Object.keys(b);
  if (aKeys.length !== bKeys.length) return false;
  return aKeys.every(
    (key) => Object.prototype.hasOwnProperty.call(b, key) && deepEqual((a as any)[key], (b as any)[key]),
  );
}
