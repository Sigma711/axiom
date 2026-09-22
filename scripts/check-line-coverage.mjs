#!/usr/bin/env node
/**
 * Release coverage gate.  Product files and React components are independent
 * obligations: a strong aggregate must not hide an untested module.
 */
import { createRequire } from 'node:module';
import { readFile, readdir } from 'node:fs/promises';
import { dirname, extname, join, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const [rustPath, browserPath] = process.argv.slice(2);
if (!rustPath || !browserPath) throw new Error('usage: check-line-coverage.mjs <rust.lcov> <browser-e2e.lcov>');

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const rustRoot = resolve(repoRoot, 'src');
const webRoot = resolve(repoRoot, 'web', 'src');
const requireFromWeb = createRequire(join(repoRoot, 'web', 'package.json'));
const ts = requireFromWeb('typescript');
const threshold = 0.95;

async function walk(dir, predicate) {
  const entries = await readdir(dir, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) files.push(...await walk(path, predicate));
    else if (predicate(path)) files.push(path);
  }
  return files;
}

function absoluteSource(source) {
  return resolve(repoRoot, source.replaceAll('\\', '/'));
}

async function collect(path) {
  const records = new Map();
  let source = '';
  for (const row of (await readFile(path, 'utf8')).split(/\r?\n/)) {
    if (row.startsWith('SF:')) { source = absoluteSource(row.slice(3)); continue; }
    if (!row.startsWith('DA:') || !source) continue;
    const [lineText, countText] = row.slice(3).split(',');
    const line = Number(lineText), count = Number(countText);
    if (!Number.isInteger(line) || line < 1 || !Number.isFinite(count) || count < 0) {
      throw new Error(`Malformed LCOV data in ${path}: ${row}`);
    }
    const lines = records.get(source) ?? new Map();
    lines.set(line, Math.max(lines.get(line) ?? 0, count));
    records.set(source, lines);
  }
  return records;
}

function ratio(lines) {
  const values = [...lines.values()];
  const covered = values.filter(count => count > 0).length;
  return { covered, total: values.length, percentage: values.length ? covered / values.length : 0 };
}

function format(result) {
  return `${result.covered}/${result.total} (${(result.percentage * 100).toFixed(2)}%)`;
}

function isInside(path, root) {
  return path === root || path.startsWith(root + sep);
}

function hasRuntimeCode(file, source) {
  if (extname(file) !== '.ts' && extname(file) !== '.tsx') return true;
  const parsed = ts.createSourceFile(file, source, ts.ScriptTarget.Latest, true, file.endsWith('.tsx') ? ts.ScriptKind.TSX : ts.ScriptKind.TS);
  let runtime = false;
  const inspect = node => {
    if (ts.isFunctionDeclaration(node) || ts.isClassDeclaration(node) || ts.isVariableStatement(node) || ts.isEnumDeclaration(node) || ts.isExpressionStatement(node)) runtime = true;
    ts.forEachChild(node, inspect);
  };
  ts.forEachChild(parsed, inspect);
  return runtime;
}

function componentRanges(file, source) {
  const tree = ts.createSourceFile(file, source, ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX);
  const components = [];
  const add = (name, node) => {
    if (!/^[A-Z]/.test(name)) return;
    const start = tree.getLineAndCharacterOfPosition(node.getStart(tree)).line + 1;
    const end = tree.getLineAndCharacterOfPosition(node.getEnd()).line + 1;
    components.push({ name, start, end });
  };
  const unwrap = node => {
    if (ts.isArrowFunction(node) || ts.isFunctionExpression(node)) return node;
    if (ts.isCallExpression(node)) return node.arguments.map(unwrap).find(Boolean);
    return undefined;
  };
  const visit = node => {
    if (ts.isFunctionDeclaration(node) && node.name) add(node.name.text, node);
    if (ts.isVariableDeclaration(node) && ts.isIdentifier(node.name)) {
      const value = node.initializer && unwrap(node.initializer);
      if (value) add(node.name.text, value);
    }
    ts.forEachChild(node, visit);
  };
  visit(tree);
  return components;
}

function componentLines(component, components, lines) {
  const nested = components.filter(other => other !== component && other.start >= component.start && other.end <= component.end);
  const data = new Map();
  for (const [line, count] of lines) {
    if (line < component.start || line > component.end) continue;
    if (nested.some(other => line >= other.start && line <= other.end)) continue;
    data.set(line, count);
  }
  return data;
}

/**
 * Rust `lib.rs`/`mod.rs` files are module manifests: they only declare or
 * re-export code that is measured in its owning source file. Likewise, the
 * knowledge reference index is generated from the source map. Neither has
 * executable product behaviour, so neither belongs in a line-coverage
 * denominator. This deliberately does not exclude ordinary application
 * modules merely because they have a low result.
 */
function isRustExecutableSource(file, source) {
  if (/^\/\/\s*由 .*自动生成/m.test(source) || /@generated|DO NOT EDIT/i.test(source)) return false;
  if (!/(?:^|\/)(?:lib|mod)\.rs$/.test(file)) return true;
  const code = source
    .replace(/\/\*[\s\S]*?\*\//g, '')
    .replace(/\/\/.*$/gm, '')
    .replace(/#!?\[[^\]]*\]/g, '')
    .trim();
  return /\b(?:fn|struct|enum|trait|impl|const|static|type|macro_rules!)\b/.test(code);
}

async function enforceRust(records) {
  const candidates = await walk(rustRoot, file => file.endsWith('.rs'));
  const files = [];
  for (const file of candidates) {
    if (isRustExecutableSource(file, await readFile(file, 'utf8'))) files.push(file);
  }
  const failures = [];
  let covered = 0, total = 0;
  for (const file of files.sort()) {
    const lines = records.get(file);
    if (!lines || lines.size === 0) {
      failures.push(`Rust module ${relative(repoRoot, file)} has no executable coverage record`);
      continue;
    }
    const result = ratio(lines); covered += result.covered; total += result.total;
    if (result.percentage < threshold) failures.push(`Rust module ${relative(repoRoot, file)}: ${format(result)}`);
  }
  const result = { covered, total, percentage: total ? covered / total : 0 };
  console.log(`Rust executable-source line coverage: ${format(result)}`);
  return failures.concat(result.percentage < threshold ? [`Rust aggregate: ${format(result)}`] : []);
}

async function enforceBrowser(records) {
  const files = await walk(webRoot, file => /\.(?:ts|tsx)$/.test(file) && !file.endsWith('.d.ts') && !/\.test\.(?:ts|tsx)$/.test(file));
  const failures = [];
  let covered = 0, total = 0;
  for (const file of files.sort()) {
    const source = await readFile(file, 'utf8');
    if (!hasRuntimeCode(file, source)) continue;
    const lines = records.get(file);
    if (!lines || lines.size === 0) {
      failures.push(`Browser product file ${relative(repoRoot, file)} has no executable coverage record`);
      continue;
    }
    const result = ratio(lines); covered += result.covered; total += result.total;
    if (result.percentage < threshold) failures.push(`Browser product file ${relative(repoRoot, file)}: ${format(result)}`);
    if (file.endsWith('.tsx')) {
      const components = componentRanges(file, source);
      for (const component of components) {
        const componentResult = ratio(componentLines(component, components, lines));
        if (componentResult.total === 0 || componentResult.percentage < threshold) {
          failures.push(`React component ${relative(repoRoot, file)}#${component.name}: ${format(componentResult)}`);
        }
      }
    }
  }
  const result = { covered, total, percentage: total ? covered / total : 0 };
  console.log(`Browser executable-source line coverage: ${format(result)}`);
  return failures.concat(result.percentage < threshold ? [`Browser aggregate: ${format(result)}`] : []);
}

const rust = await collect(rustPath);
const browser = await collect(browserPath);
const failures = [...await enforceRust(rust), ...await enforceBrowser(browser)];
if (failures.length) {
  console.error('\nCoverage gate failures:');
  for (const failure of failures) console.error(`- ${failure}`);
  throw new Error(`${failures.length} independent coverage requirement(s) are below 95.00%`);
}
