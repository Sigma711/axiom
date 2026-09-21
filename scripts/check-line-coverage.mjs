#!/usr/bin/env node
import { readFile } from 'node:fs/promises';

const [rustPath, browserPath] = process.argv.slice(2);
if (!rustPath || !browserPath) {
  throw new Error('usage: check-line-coverage.mjs <rust.lcov> <browser.lcov>');
}

async function collect(path) {
  const records = new Map();
  let source = '';
  for (const row of (await readFile(path, 'utf8')).split(/\r?\n/)) {
    if (row.startsWith('SF:')) source = row.slice(3);
    if (!row.startsWith('DA:') || !source) continue;
    const [line, count] = row.slice(3).split(',').map(Number);
    const key = `${source}:${line}`;
    records.set(key, Math.max(records.get(key) ?? 0, count));
  }
  return records;
}

const merged = new Map([...(await collect(rustPath)), ...(await collect(browserPath))]);
const covered = [...merged.values()].filter((count) => count > 0).length;
const total = merged.size;
const percentage = total === 0 ? 0 : covered / total * 100;
console.log(`Verified executable-source line coverage: ${covered}/${total} (${percentage.toFixed(2)}%)`);
if (percentage < 95) {
  throw new Error(`coverage ${percentage.toFixed(2)}% is below the required 95.00%`);
}
