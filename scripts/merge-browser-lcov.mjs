#!/usr/bin/env node
/**
 * Merge browser unit and real-Chromium LCOV by source line. Both are product
 * tests: the union preserves unit-tested helpers and E2E-tested interactions.
 */
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { dirname, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const [unitPath, e2ePath, outputPath] = process.argv.slice(2);
if (!unitPath || !e2ePath || !outputPath) throw new Error('usage: merge-browser-lcov.mjs <unit.lcov> <e2e.lcov> <output.lcov>');
const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');

function normalize(source) {
  const posix = source.replaceAll('\\\\', '/');
  return posix.startsWith('src/') ? 'web/' + posix : posix.replace(/^\.\//, '');
}
async function collect(path) {
  const files = new Map();
  let source = '';
  for (const row of (await readFile(path, 'utf8')).split(/\r?\n/)) {
    if (row.startsWith('SF:')) {
      source = normalize(row.slice(3));
      if (!files.has(source)) files.set(source, new Map());
    } else if (row.startsWith('DA:') && source) {
      const [lineText, countText] = row.slice(3).split(',');
      const line = Number(lineText), count = Number(countText);
      if (Number.isInteger(line) && line > 0 && Number.isFinite(count) && count >= 0) {
        const lines = files.get(source);
        lines.set(line, Math.max(lines.get(line) ?? 0, count));
      }
    }
  }
  return files;
}
function stats(files) {
  let covered = 0, total = 0;
  for (const lines of files.values()) {
    total += lines.size;
    covered += [...lines.values()].filter(count => count > 0).length;
  }
  return { covered, total };
}
const [unit, e2e] = await Promise.all([collect(unitPath), collect(e2ePath)]);
const combined = new Map([...unit].map(([source, lines]) => [source, new Map(lines)]));
for (const [source, incoming] of e2e) {
  const lines = combined.get(source) ?? new Map();
  for (const [line, count] of incoming) lines.set(line, Math.max(lines.get(line) ?? 0, count));
  combined.set(source, lines);
}
const rows = [];
for (const source of [...combined.keys()].sort()) {
  rows.push('TN:', 'SF:' + source);
  for (const [line, count] of [...combined.get(source)].sort(([a], [b]) => a - b)) rows.push('DA:' + line + ',' + count);
  rows.push('end_of_record');
}
await mkdir(dirname(resolve(outputPath)), { recursive: true });
await writeFile(outputPath, rows.join('\n') + '\n');
for (const [label, files] of [['Unit', unit], ['Playwright V8', e2e], ['Combined browser', combined]]) {
  const { covered, total } = stats(files);
  console.log(label + ' coverage: ' + covered + '/' + total + ' executable lines across ' + files.size + ' product files');
}
console.log('LCOV: ' + relative(repoRoot, resolve(outputPath)));
