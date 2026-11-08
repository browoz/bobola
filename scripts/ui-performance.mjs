import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { chromium } from 'playwright';

const label = process.argv[2] ?? 'after';
assert.match(label, /^[a-z-]+$/);
const output = `output/ui-${label}`;
await mkdir(output, { recursive: true });
const server = spawn(
  process.execPath,
  [
    'node_modules/vite/bin/vite.js',
    'preview',
    '--host',
    '127.0.0.1',
    '--port',
    '1437',
    '--strictPort',
  ],
  { stdio: 'ignore' }
);
let browser;
try {
  for (let i = 0; i < 50; i++) {
    try {
      if ((await fetch('http://127.0.0.1:1437')).ok) break;
    } catch {}
    await new Promise((resolve) => setTimeout(resolve, 200));
  }
  browser = await chromium.launch({ channel: 'msedge', headless: true });
  const results = [];
  for (const scale of [1, 1.5, 2]) {
    const page = await browser.newPage({
      viewport: { width: 1320, height: 860 },
      deviceScaleFactor: scale,
    });
    await page.addInitScript(() => {
      if (localStorage.getItem('service-workspace-preview-v1')) return;
      localStorage.setItem(
        'service-workspace-preview-v1',
        JSON.stringify({
          version: 1,
          preferences: { liveBudget: 5, idleMinutes: 15, theme: 'light' },
          workspaces: [{ id: 'personal', name: 'Personal' }],
          services: [
            'WhatsApp',
            'Telegram',
            'Discord',
            'Slack',
            'Gmail',
            'Notion',
            'GitHub',
            'YouTube',
          ].map((name, i) => ({
            id: `perf-${i}`,
            name,
            url: 'https://example.com',
            workspaceId: 'personal',
            profileName: `account_perf-${i}`,
            keepLive: true,
            protection: true,
          })),
        })
      );
      localStorage.setItem(
        'service-workspace-preview-v1-interface',
        JSON.stringify({ navMode: 'pinned', theme: 'light', companion: { enabled: false } })
      );
    });
    await page.goto('http://127.0.0.1:1437');
    await page.getByRole('button', { name: 'Manage WhatsApp', exact: true }).waitFor();
    const startup = await page.evaluate(() => performance.now());
    const timings = await page.evaluate(async () => {
      const frame = () => new Promise((resolve) => requestAnimationFrame(resolve));
      const switches = [],
        frames = [];
      for (let round = 0; round < 3; round++)
        for (const name of [
          'WhatsApp',
          'Telegram',
          'Discord',
          'Slack',
          'Gmail',
          'Notion',
          'GitHub',
          'YouTube',
        ]) {
          const start = performance.now();
          document.querySelector(`.service-item > button[title="${name}"]`).click();
          await frame();
          await frame();
        switches.push(performance.now() - start);
        const slot = document.querySelector('[data-service-id]');
        if (!slot || !slot.textContent.includes(name)) throw new Error(`Switch failed: ${name}`);
        }
      let previous = await frame();
      for (let i = 0; i < 60; i++) {
        const now = await frame();
        frames.push(now - previous);
        previous = now;
      }
      return { switches, frames };
    });
    for (const theme of ['light', 'dark']) {
      await page.evaluate(() => {
        const key = 'service-workspace-preview-v1';
        const data = JSON.parse(localStorage.getItem(key));
        const ids = ['whatsapp', 'telegram', 'discord', 'slack', 'gmail', 'notion', 'github', 'youtube'];
        data.services.forEach((service, i) => service.catalogId = ids[i]);
        localStorage.setItem(key, JSON.stringify(data));
      });
      await page.evaluate((theme) => {
        const key = 'service-workspace-preview-v1-interface';
        localStorage.setItem(
          key,
          JSON.stringify({ ...JSON.parse(localStorage.getItem(key)), theme })
        );
      }, theme);
      await page.reload();
      await page.getByRole('button', { name: 'Manage WhatsApp', exact: true }).waitFor();
      await page.screenshot({ path: `${output}/${theme}-${scale}x.png` });
      if (scale === 1) {
        await page.evaluate(() => {
          const key = 'service-workspace-preview-v1-interface';
          localStorage.setItem(key, JSON.stringify({ ...JSON.parse(localStorage.getItem(key)), navMode: 'minimal' }));
        });
        await page.reload();
        await page.getByRole('button', { name: 'Quick switch', exact: true }).waitFor();
        await page.screenshot({ path: `${output}/${theme}-rail.png` });
        await page.evaluate(() => {
          const key = 'service-workspace-preview-v1-interface';
          localStorage.setItem(key, JSON.stringify({ ...JSON.parse(localStorage.getItem(key)), navMode: 'pinned' }));
        });
        await page.reload();
      }
      for (const viewport of [
        { width: 900, height: 640 },
        { width: 1600, height: 1000 },
      ]) {
        await page.setViewportSize(viewport);
        assert.ok(
          await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth),
          'shell fits resized viewport'
        );
      }
      await page.setViewportSize({ width: 1320, height: 860 });
    }
    results.push({ scale, startupToAutomationReadyMs: startup, ...timings });
    await page.close();
  }
  await writeFile(
    `${output}/measurements.json`,
    JSON.stringify(
      {
        kind: 'Edge headless shell preview; eight saved services, no native websites',
        generatedAt: new Date().toISOString(),
        results,
      },
      null,
      2
    )
  );
  console.log(
    `PASS: eight-service switching, light/dark, resizing, 100/150/200% browser scale. Evidence: ${output}`
  );
} finally {
  await browser?.close();
  server.kill();
}
