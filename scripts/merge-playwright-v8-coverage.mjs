#!/usr/bin/env node
/**
 * Convert Playwright's Chromium precise V8 data to Istanbul LCOV.
 *
 * Every V8 range is interpreted by v8-to-istanbul, which understands nested
 * ranges and inline source maps.  Do not replace this with a max(range.count)
 * merge: an executed outer module range must never credit an unexecuted inner
 * branch or component.
 */
import { createRequire } from 'node:module';
import { mkdir, readFile, readdir } from 'node:fs/promises';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const inputRoot = resolve(process.argv[2] ?? join(repoRoot, 'web', 'test-results'));
const outputDir = resolve(process.argv[3] ?? join(repoRoot, 'coverage', 'web-e2e'));
const webSrcRoot = resolve(repoRoot, 'web', 'src');
const requireFromWeb = createRequire(join(repoRoot, 'web', 'package.json'));
const v8ToIstanbul = requireFromWeb('v8-to-istanbul');
const libCoverage = requireFromWeb('istanbul-lib-coverage');
const libReport = requireFromWeb('istanbul-lib-report');
const reports = requireFromWeb('istanbul-reports');

async function findCoverageFiles(dir) {
  let entries;
  try { entries = await readdir(dir, { withFileTypes: true }); } catch { return []; }
  const files = [];
  for (const entry of entries) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) files.push(...await findCoverageFiles(path));
    else if (entry.name === 'v8-coverage.json') files.push(path);
  }
  return files;
}

function sourcePath(scriptUrl) {
  try {
    const pathname = decodeURIComponent(new URL(scriptUrl).pathname);
    const marker = '/src/';
    const start = pathname.lastIndexOf(marker);
    if (start >= 0) {
      const candidate = resolve(webSrcRoot, pathname.slice(start + marker.length));
      if (candidate.startsWith(webSrcRoot) && /\.(?:ts|tsx)$/.test(candidate)) return candidate;
    }
  } catch { /* validated below */ }
  return null;
}

const files = await findCoverageFiles(inputRoot);
if (files.length === 0) throw new Error(`No Playwright V8 coverage files found below ${relative(repoRoot, inputRoot)}`);
const coverageMap = libCoverage.createCoverageMap({});
let scripts = 0;
let skipped = 0;
for (const file of files) {
  const payload = JSON.parse(await readFile(file, 'utf8'));
  for (const script of payload.scripts ?? []) {
    const path = sourcePath(script.url);
    if (!path || !script.source || !Array.isArray(script.functions)) { skipped += 1; continue; }
    const converter = v8ToIstanbul(path, 0, { source: script.source });
    await converter.load();
    converter.applyCoverage(script.functions);
    coverageMap.merge(converter.toIstanbul());
    scripts += 1;
  }
}
if (scripts === 0) throw new Error('V8 reports contained no mappable web/src TypeScript scripts');
await mkdir(outputDir, { recursive: true });
const context = libReport.createContext({ dir: outputDir, coverageMap });
reports.create('lcovonly', { file: 'lcov.info', projectRoot: repoRoot }).execute(context);
let executable = 0, covered = 0;
for (const file of coverageMap.files()) {
  const lines = coverageMap.fileCoverageFor(file).getLineCoverage();
  executable += Object.keys(lines).length;
  covered += Object.values(lines).filter(count => count > 0).length;
}
console.log(`Playwright V8 coverage: ${covered}/${executable} executable lines across ${coverageMap.files().length} observed web files; ignored ${skipped} non-TypeScript scripts.`);
console.log(`LCOV: ${relative(repoRoot, join(outputDir, 'lcov.info'))}`);
