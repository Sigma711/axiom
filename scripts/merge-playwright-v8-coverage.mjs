#!/usr/bin/env node
import { createRequire } from 'node:module';
import { mkdir, readFile, readdir, writeFile } from 'node:fs/promises';
import { dirname, isAbsolute, join, normalize, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const inputRoot = resolve(process.argv[2] ?? join(repoRoot, 'web', 'test-results'));
const outputPath = resolve(process.argv[3] ?? join(repoRoot, 'coverage', 'web-e2e', 'lcov.info'));
const webSrcRoot = resolve(repoRoot, 'web', 'src');
const requireFromWeb = createRequire(join(repoRoot, 'web', 'package.json'));
const { SourceMapConsumer } = requireFromWeb('source-map-js');

const lineHits = new Map();

async function walk(dir) {
  const entries = await readdir(dir, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    const current = join(dir, entry.name);
    if (entry.isDirectory()) files.push(...await walk(current));
    else if (/\.(?:ts|tsx)$/.test(entry.name) && !entry.name.endsWith('.d.ts') && !/\.test\.(?:ts|tsx)$/.test(entry.name)) files.push(current);
  }
  return files;
}

async function findCoverageFiles(dir) {
  let entries;
  try {
    entries = await readdir(dir, { withFileTypes: true });
  } catch {
    return [];
  }
  const files = [];
  for (const entry of entries) {
    const current = join(dir, entry.name);
    if (entry.isDirectory()) files.push(...await findCoverageFiles(current));
    else if (entry.name === 'v8-coverage.json') files.push(current);
  }
  return files;
}

function lineStarts(text) {
  const starts = [0];
  for (let index = 0; index < text.length; index += 1) if (text[index] === '\n') starts.push(index + 1);
  return starts;
}

function lineForOffset(starts, offset) {
  let low = 0;
  let high = starts.length;
  while (low + 1 < high) {
    const middle = (low + high) >> 1;
    if (starts[middle] <= offset) low = middle;
    else high = middle;
  }
  return low;
}

function sourceMapFromScript(source) {
  const match = source.match(/\/\/# sourceMappingURL=(data:application\/json[^\s]+|[^\s]+)/);
  if (!match) return null;
  const value = match[1];
  if (value.startsWith('data:')) {
    const comma = value.indexOf(',');
    if (comma < 0) return null;
    const body = value.slice(comma + 1);
    try {
      return JSON.parse(value.slice(0, comma).includes(';base64') ? Buffer.from(body, 'base64').toString('utf8') : decodeURIComponent(body));
    } catch {
      return null;
    }
  }
  return null;
}

function originalPath(source, sourceMap, sourceIndex, scriptUrl) {
  const sourceName = sourceMap?.sources?.[sourceIndex];
  if (!sourceName) return null;
  const sourceRoot = sourceMap.sourceRoot ?? '';
  let candidate = sourceName;
  if (sourceRoot) candidate = `${sourceRoot.replace(/\/$/, '')}/${candidate}`;
  if (candidate.startsWith('file://')) {
    try { candidate = fileURLToPath(candidate); } catch { return null; }
  } else if (/^https?:\/\//.test(candidate)) {
    try { candidate = new URL(candidate).pathname; } catch { return null; }
  } else if (!isAbsolute(candidate)) {
    try { candidate = new URL(candidate, scriptUrl).pathname; } catch { /* handled below */ }
  }
  candidate = normalize(candidate);
  const marker = `${normalize('/src')}${'/'}`;
  const srcMarker = candidate.replaceAll('\\', '/').lastIndexOf('/src/');
  if (srcMarker >= 0) return resolve(webSrcRoot, candidate.replaceAll('\\', '/').slice(srcMarker + marker.length));
  if (candidate.startsWith(webSrcRoot)) return candidate;
  return null;
}

function record(file, line, count) {
  if (!file || line < 1 || !file.startsWith(webSrcRoot)) return;
  let lines = lineHits.get(file);
  if (!lines) { lines = new Map(); lineHits.set(file, lines); }
  lines.set(line, Math.max(lines.get(line) ?? 0, count));
}

function mapScript(script) {
  if (!script.source || !script.functions?.length) return;
  const starts = lineStarts(script.source);
  const sourceMap = sourceMapFromScript(script.source);
  let consumer = null;
  try { if (sourceMap) consumer = new SourceMapConsumer(sourceMap); } catch { consumer = null; }
  try {
    for (const fn of script.functions) {
      for (const range of fn.ranges) {
        if (range.endOffset <= range.startOffset) continue;
        const startLine = lineForOffset(starts, range.startOffset);
        const endLine = lineForOffset(starts, range.endOffset - 1);
        for (let generatedLine = startLine; generatedLine <= endLine; generatedLine += 1) {
          const generatedColumn = generatedLine === startLine ? range.startOffset - starts[generatedLine] : 0;
          if (!consumer) continue;
          const mapped = consumer.originalPositionFor({ line: generatedLine + 1, column: generatedColumn, bias: SourceMapConsumer.GREATEST_LOWER_BOUND });
          const file = mapped.source ? originalPath(mapped.source, { sources: [mapped.source], sourceRoot: '' }, 0, script.url) : null;
          if (file && mapped.line) record(file, mapped.line, range.count);
        }
      }
    }
  } finally {
    consumer?.destroy?.();
  }
}

const coverageFiles = await findCoverageFiles(inputRoot);
for (const coverageFile of coverageFiles) {
  let payload;
  try { payload = JSON.parse(await readFile(coverageFile, 'utf8')); } catch { continue; }
  for (const script of payload.scripts ?? []) mapScript(script);
}

const sourceFiles = await walk(webSrcRoot);
const lcov = [];
let totalLines = 0;
let coveredLines = 0;
for (const file of sourceFiles.sort()) {
  const hits = lineHits.get(file) ?? new Map();
  // V8 mappings identify executable original lines; type-only declarations and
  // comments have no generated range and therefore stay out of the denominator.
  const executableLines = [...hits.keys()].sort((a, b) => a - b);
  const fileCovered = executableLines.filter((line) => (hits.get(line) ?? 0) > 0).length;
  totalLines += executableLines.length;
  coveredLines += fileCovered;
  lcov.push(`TN:`, `SF:${file}`);
  for (const line of executableLines) lcov.push(`DA:${line},${hits.get(line) ?? 0}`);
  lcov.push(`LF:${executableLines.length}`, `LH:${fileCovered}`, 'end_of_record');
}
await mkdir(dirname(outputPath), { recursive: true });
await writeFile(outputPath, `${lcov.join('\n')}\n`, 'utf8');
const percent = totalLines ? (coveredLines / totalLines * 100).toFixed(2) : '0.00';
console.log(`Playwright V8 coverage: ${coveredLines}/${totalLines} executable lines (${percent}%) across ${sourceFiles.length} web/src files`);
console.log(`LCOV: ${relative(repoRoot, outputPath)}`);
