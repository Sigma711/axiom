#!/usr/bin/env node
import { createRequire } from 'node:module';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const requireFromWeb = createRequire(join(repoRoot, 'web', 'package.json'));
const { build } = requireFromWeb('vite');

await build({
  root: join(repoRoot, 'web'),
  configFile: join(repoRoot, 'web', 'vite.config.ts'),
  build: { sourcemap: 'inline' },
});

console.log('Built web/static with inline source maps for browser coverage.');
