import { defineConfig, devices } from '@playwright/test';

const realService = process.env.AXIOM_REAL === '1';

export default defineConfig({
  testDir: './e2e',
  reporter: [['list'], ['html', { open: 'never' }]],
  preserveOutput: 'always',
  testMatch: realService ? /real\.spec\.ts/ : /app\.spec\.ts/,
  timeout: 30_000,
  use: { baseURL: realService ? 'http://127.0.0.1:18080' : 'http://127.0.0.1:18181', viewport: { width: 1440, height: 1000 }, colorScheme: 'dark', timezoneId: 'UTC', locale: 'en-US' },
  webServer: realService
    ? { command: 'cd .. && make serve-test', url: 'http://127.0.0.1:18080', reuseExistingServer: false }
    : { command: 'VITE_BASE=/ npm run dev -- --host 127.0.0.1 --port 18181', url: 'http://127.0.0.1:18181', reuseExistingServer: !process.env.CI },
  projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }],
});
