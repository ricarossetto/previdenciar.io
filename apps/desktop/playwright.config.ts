import { defineConfig } from '@playwright/test';
const executablePath = process.env.PLAYWRIGHT_EXECUTABLE_PATH;
export default defineConfig({
  testDir: './tests/e2e', timeout: 45000, workers: 1, retries: 0,
  reporter: [['list'], ['json', { outputFile: 'verification/playwright-report.json' }]],
  use: { baseURL: 'http://127.0.0.1:1427', viewport: { width: 1440, height: 1000 },
    launchOptions: executablePath ? { executablePath } : {}, screenshot: 'only-on-failure', trace: 'retain-on-failure' },
  webServer: { command: 'npm run build && npm run preview', url: 'http://127.0.0.1:1427', reuseExistingServer: false, timeout: 120000 },
});
