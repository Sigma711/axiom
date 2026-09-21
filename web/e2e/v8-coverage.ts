import { mkdir, writeFile } from 'node:fs/promises';
import { dirname } from 'node:path';
import { expect, test as base, type Page } from '@playwright/test';

type CoverageFixtures = {
  v8Coverage: void;
};

type CoverageScript = {
  scriptId: string;
  url: string;
  functions: Array<{
    functionName: string;
    ranges: Array<{ startOffset: number; endOffset: number; count: number }>;
    isBlockCoverage: boolean;
  }>;
  source: string;
};

type CoverageResult = {
  result: Array<Omit<CoverageScript, 'source'>>;
};

type DebuggerSource = { scriptSource: string };

const enabled = process.env.PW_V8_COVERAGE === '1';

function isApplicationScript(url: string) {
  return (url.includes('/src/') && !url.includes('/node_modules/')) || url.includes('/static/assets/');
}

export const test = base.extend<CoverageFixtures>({
  v8Coverage: [async ({ page }, use, testInfo) => {
    if (!enabled) {
      await use();
      return;
    }

    const outputPath = testInfo.outputPath('v8-coverage.json');
    const client = await page.context().newCDPSession(page);
    await client.send('Debugger.enable');
    await client.send('Profiler.enable');
    await client.send('Profiler.startPreciseCoverage', { callCount: true, detailed: true });
    try {
      await use();
    } finally {
      const coverage = await client.send('Profiler.takePreciseCoverage') as CoverageResult;
      await client.send('Profiler.stopPreciseCoverage');
      await client.send('Profiler.disable');
      const scripts: CoverageScript[] = [];
      for (const script of coverage.result) {
        if (!isApplicationScript(script.url)) continue;
        let source = '';
        try {
          source = (await client.send('Debugger.getScriptSource', { scriptId: script.scriptId }) as DebuggerSource).scriptSource;
        } catch {
          // A script can disappear during a navigation; its other coverage still remains useful.
        }
        scripts.push({ ...script, source });
      }
      await mkdir(dirname(outputPath), { recursive: true });
      await writeFile(outputPath, JSON.stringify({ testTitle: testInfo.title, scripts }), 'utf8');
    }
  }, { auto: true }],
});

export { expect };
export type { Page };
