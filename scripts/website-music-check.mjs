import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir } from 'node:fs/promises';
import { chromium } from 'playwright';

// Owned browser-preview data: exercise the replacement service flow and Canvas.
const origin = 'http://127.0.0.1:1438';
const server = spawn(
  process.execPath,
  [
    'node_modules/vite/bin/vite.js',
    'preview',
    '--host',
    '127.0.0.1',
    '--port',
    '1438',
    '--strictPort',
  ],
  { stdio: 'ignore' }
);
let browser;
try {
  for (let attempt = 0; ; attempt++) {
    try {
      if ((await fetch(origin)).ok) break;
    } catch {}
    if (attempt === 50) throw new Error('Preview did not start');
    await new Promise((resolve) => setTimeout(resolve, 200));
  }
  browser = await chromium.launch({ channel: 'msedge', headless: true });
  const page = await browser.newPage({ viewport: { width: 1320, height: 860 } });
  await page.goto(origin);
  await page.getByRole('button', { name: 'Open QuickDraw canvas', exact: true }).waitFor();
  assert.equal(await page.getByRole('button', { name: 'Open Music', exact: true }).count(), 0);
  await page.getByRole('button', { name: 'Open QuickDraw canvas', exact: true }).click();
  await page.getByRole('region', { name: 'Bobola QuickDraw canvas', exact: true }).waitFor();
  await page.keyboard.press('Control+k');
  await page.getByRole('dialog').getByRole('button', { name: 'Add service', exact: true }).click();
  await page.getByLabel('Find an app').fill('YouTube Music');
  await page
    .locator('.catalog')
    .getByRole('button', { name: 'YouTube Music', exact: true })
    .click();
  assert.equal(await page.getByLabel('Website URL').inputValue(), 'https://music.youtube.com/');
  await page.getByRole('dialog').getByRole('button', { name: 'Add service', exact: true }).click();
  await page.locator('.preview-placeholder').waitFor();
  const saved = await page.evaluate(
    () => JSON.parse(localStorage.getItem('service-workspace-preview-v1')).services
  );
  assert.equal(saved.length, 1);
  assert.equal(saved[0].catalogId, 'youtubemusic');
  assert.equal(saved[0].url, 'https://music.youtube.com/');
  assert.equal(await page.locator('.nav img[src="/service-icons/youtubemusic.svg"]').count(), 1);
  await page.evaluate(() => {
    const key = 'service-workspace-preview-v1-interface';
    const saved = JSON.parse(localStorage.getItem(key) ?? '{}');
    localStorage.setItem(key, JSON.stringify({ ...saved, navMode: 'glass-dock' }));
  });
  await page.reload();
  const dock = page.getByRole('toolbar', { name: 'Floating Glass Dock' });
  await dock.waitFor();
  assert.equal(await dock.getByRole('button', { name: 'Open Music', exact: true }).count(), 0);
  await dock.getByRole('button', { name: 'YouTube Music', exact: true }).click();
  await page.locator('.preview-placeholder').waitFor();
  await mkdir('output/website-music', { recursive: true });
  await page.screenshot({ path: 'output/website-music/service.png' });
  await dock.getByRole('button', { name: 'Open Canvas', exact: true }).click();
  await page.getByRole('region', { name: 'Bobola QuickDraw canvas', exact: true }).waitFor();
  await page.screenshot({ path: 'output/website-music/canvas.png' });
  console.log(
    'PASS: native Music removed from rail and dock; YouTube Music adds, persists, and opens as a website service; Canvas opens in both navigation modes.'
  );
} finally {
  await browser?.close();
  server.kill();
}
