import { expect, it } from 'vitest';
import { appBase, appPath } from './api';

it('keeps application requests and routes under Vite deployment base', () => {
  expect(appBase === '' || appBase.startsWith('/')).toBe(true);
  expect(appPath('/api/knowledge')).toBe(`${appBase}/api/knowledge`);
  expect(appPath('/backtest')).toBe(`${appBase}/backtest`);
});
