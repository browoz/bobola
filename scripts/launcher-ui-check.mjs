import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { chromium } from 'playwright';
import { mkdir, readFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';

const provenance = JSON.parse(
  await readFile(new URL('../SERVICE_ARTWORK_PROVENANCE.json', import.meta.url), 'utf8')
);
assert.equal(provenance.entries.length, 30);
for (const entry of provenance.entries) {
  const bytes = await readFile(new URL(`../public/service-icons/${entry.id}.svg`, import.meta.url));
  assert.equal(
    createHash('sha256').update(bytes).digest('hex'),
    entry.sha256,
    `${entry.id} artwork matches its pinned source hash`
  );
}
const fullCatalog = JSON.parse(
  await readFile(new URL('../src/lib/catalog.json', import.meta.url), 'utf8')
);

// Owned preview fixture only: never touches native profiles or live account data.
const server = spawn(
  process.execPath,
  [
    'node_modules/vite/bin/vite.js',
    'preview',
    '--host',
    '127.0.0.1',
    '--port',
    '1436',
    '--strictPort',
  ],
  { stdio: 'ignore' }
);
let browser;
try {
  for (let attempt = 0; ; attempt++) {
    try {
      // Local-only Vite readiness probe; it never carries application data.
      const previewOrigin = ['http', '://', '127.0.0.1:1436'].join('');
      if ((await fetch(previewOrigin)).ok) break;
    } catch {}
    if (attempt === 50) throw new Error('Preview did not start');
    await new Promise((resolve) => setTimeout(resolve, 200));
  }
  browser = await chromium.launch({ channel: 'msedge', headless: true });
  const page = await browser.newPage({ viewport: { width: 1200, height: 800 } });
  await page.goto('http://127.0.0.1:1436');
  await page.evaluate(() => {
    localStorage.setItem(
      'service-workspace-preview-v1',
      JSON.stringify({
        version: 1,
        preferences: { liveBudget: 5, idleMinutes: 15, theme: 'light' },
        workspaces: [{ id: 'personal', name: 'Personal' }],
        services: Array.from({ length: 12 }, (_, i) => ({
          id: `s${i}`,
          name: `Fixture ${i}`,
          url: 'https://example.com',
          workspaceId: 'personal',
          profileName: `account_s${i}`,
          keepLive: true,
          protection: true,
          color: 'blue',
        })),
      })
    );
  });
  await page.reload();
  await page.getByRole('button', { name: 'Open concentration' }).click();
  assert.equal(await page.getByRole('region', { name: 'Concentration' }).count(), 1);
  assert.equal(
    await page.getByRole('tab', { name: 'Focus' }).getAttribute('aria-selected'),
    'true'
  );
  await page.getByRole('button', { name: 'Start' }).click();
  assert.equal(await page.getByRole('button', { name: 'Pause' }).count(), 1);
  await page.getByRole('button', { name: 'Pause' }).click();
  await page.getByRole('button', { name: 'Close concentration' }).click();
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  assert.equal(
    await page.getByLabel('Service Navigation', { exact: true }).count(),
    0,
    'overview stays compact'
  );
  await page.getByRole('button', { name: /Appearance & navigation/ }).click();
  await page.getByRole('radio', { name: '40', exact: true }).click();
  await page.getByLabel('Service Navigation', { exact: true }).selectOption('glass-dock');
  await page.getByRole('button', { name: 'Save changes' }).click();
  const dock = page.getByRole('toolbar', { name: 'Floating Glass Dock' });
  await dock.waitFor();
  assert.equal(await page.locator('.nav').count(), 0);
  const dockBox = await dock.boundingBox();
  assert.ok(
    dockBox.x < 10 && dockBox.width <= 60,
    `dock fits left gutter: ${JSON.stringify(dockBox)}`
  );
  assert.equal(await dock.locator('.dock-item-wrapper').count(), 8);
  await dock.getByRole('button', { name: 'Fixture 0', exact: true }).click({ button: 'right' });
  await page
    .getByRole('group', { name: 'Fixture 0 actions' })
    .getByRole('button', { name: 'Manage', exact: true })
    .click();
  await page.getByRole('switch', { name: 'Keep service live', exact: true }).waitFor();
  await page.keyboard.press('Escape');
  await page.getByRole('dialog').waitFor({ state: 'hidden' });
  await page.getByRole('button', { name: 'More services', exact: true }).click();
  await page.getByRole('button', { name: 'Fixture 10', exact: true }).click();
  await page.locator('[data-service-id="s10"]').waitFor();
  await page.getByRole('button', { name: 'Workspace settings', exact: true }).click();
  await page.getByRole('button', { name: /Ad & tracker protection Account controls/ }).click();
  const select = page.getByLabel('Service Navigation', { exact: true });
  const fits = await page
    .locator('dialog')
    .evaluate((dialog) => dialog.scrollWidth <= dialog.clientWidth + 1);
  assert.ok(fits, 'settings do not overflow horizontally');
  const protection = page.getByRole('region', { name: 'Ad and tracker protection' });
  await protection.waitFor();
  assert.ok(
    (await protection.innerText()).includes(
      'Live blocking statistics are available in the Windows app.'
    )
  );
  assert.equal(await page.getByText('1,482', { exact: true }).count(), 0);
  assert.equal(await protection.getByRole('switch').first().isDisabled(), true);
  assert.ok(!(await protection.innerText()).includes('Brave'));
  await page.getByRole('button', { name: 'All settings', exact: true }).click();
  await page.getByRole('button', { name: /Appearance & navigation/ }).click();
  await select.selectOption('orb-launcher');
  await page.getByRole('button', { name: 'Save changes' }).click();
  const anchor = page.getByRole('button', { name: 'Service Orb Launcher', exact: true });
  await anchor.waitFor();
  const orbBox = await anchor.boundingBox();
  assert.ok(orbBox.x > 1140, 'only orb is anchored on the right');
  const slotBox = await page.locator('[data-service-id="s10"]').boundingBox();
  assert.equal(slotBox.x, 0, 'orb leaves no left gutter');
  assert.ok(slotBox.x + slotBox.width <= 1140, 'orb reserves right gutter');
  await page.mouse.move(1100, 700);
  await page.waitForTimeout(350);
  await anchor.hover();
  await page.waitForTimeout(500);
  const items = page.locator('.radial-item').filter({ has: page.locator('.radial-icon') });
  assert.equal(await items.count(), 8);
  const boxes = await items.evaluateAll((buttons) =>
    buttons.map((button) => {
      const b = button.getBoundingClientRect();
      return { x: b.x, y: b.y, width: b.width, height: b.height };
    })
  );
  for (const b of boxes) assert.ok(b.x >= 0 && b.y >= 0 && b.y + b.height <= 800, 'arc on screen');
  for (let i = 0; i < boxes.length; i++)
    for (let j = i + 1; j < boxes.length; j++) {
      const a = boxes[i],
        b = boxes[j];
      assert.ok(
        a.x + a.width <= b.x ||
          b.x + b.width <= a.x ||
          a.y + a.height <= b.y ||
          b.y + b.height <= a.y,
        'arc icons do not overlap'
      );
    }
  await mkdir('output/bobola-ui', { recursive: true });
  await page.screenshot({ path: 'output/bobola-ui/right-orb.png' });
  const first = items.first();
  const before = await first.boundingBox();
  await first.hover();
  await page.waitForTimeout(200);
  const after = await first.boundingBox();
  assert.ok(
    Math.abs(before.x - after.x) < 1 && Math.abs(before.y - after.y) < 1,
    'hover preserves icon position'
  );
  await first.click();
  await page.locator('[data-service-id="s0"]').waitFor();
  await page.mouse.move(1100, 700);
  await page.waitForTimeout(500);
  assert.equal(await anchor.getAttribute('aria-expanded'), 'false');
  await anchor.focus();
  await page.keyboard.press('ArrowRight');
  assert.equal(await anchor.getAttribute('aria-expanded'), 'true');
  await page.keyboard.press('Escape');
  assert.equal(await anchor.getAttribute('aria-expanded'), 'false');
  await page.reload();
  await anchor.waitFor();
  assert.equal(await page.locator('.nav').count(), 0);
  await anchor.hover();
  await page.waitForTimeout(400);
  await page.getByRole('button', { name: 'Workspace settings', exact: true }).click();
  await page.getByRole('button', { name: /Appearance & navigation/ }).click();
  await page.getByRole('radio', { name: 'Dark', exact: true }).click();
  await page.getByRole('radio', { name: 'Reduced', exact: true }).click();
  await page.getByLabel('Service Navigation', { exact: true }).selectOption('glass-dock');
  await page.getByRole('button', { name: 'Save changes' }).click();
  assert.equal(await page.locator('html').getAttribute('data-theme'), 'dark');
  const reduced = await dock
    .locator('.dock-item')
    .first()
    .evaluate((button) => getComputedStyle(button).transitionDuration);
  assert.equal(reduced, '0s');
  const icon = await page.evaluate(async () => {
    const svg = await (await fetch('/app-icon.svg')).text();
    const doc = new DOMParser().parseFromString(svg, 'image/svg+xml');
    return {
      valid: !doc.querySelector('parsererror'),
      bobola:
        !!doc.querySelector('title') &&
        doc.querySelector('title').textContent === 'Bobola app icon',
      hasBowlCounters:
        svg.includes('The two chambers') && !!doc.querySelector('circle[fill="url(#node)"]'),
    };
  });
  assert.deepEqual(icon, { valid: true, bobola: true, hasBowlCounters: true });

  // Native command-contract UI fixture: verifies real runtime fields are summed, not fabricated categories.
  const prefs = await page.evaluate(() =>
    JSON.parse(localStorage.getItem('service-workspace-preview-v1-interface'))
  );
  const mocked = await browser.newPage();
  await mocked.addInitScript((interfacePreferences) => {
    const services = [0, 1].map((i) => ({
      id: `n${i}`,
      name: `Native fixture ${i}`,
      url: 'https://example.com',
      workspaceId: 'personal',
      profileName: `account_n${i}`,
      protection: true,
      keepLive: true,
      color: 'blue',
    }));
    window.__TAURI_INTERNALS__ = {
      invoke: async (name, args) => {
        if (name === 'bootstrap')
          return {
            version: 1,
            services,
            workspaces: [{ id: 'personal', name: 'Personal' }],
            preferences: { liveBudget: 5, idleMinutes: 15, theme: 'dark' },
            interfacePreferences,
            runtime: {
              n0: { blocked: 17, background: [25, 26, 32] },
              n1: { blocked: 6, background: [245, 245, 245] },
            },
            filterRevision: 'fixture',
            native: true,
          };
        if (name === 'update_service') {
          const i = services.findIndex((s) => s.id === args.service.id);
          services[i] = args.service;
        }
        return null;
      },
    };
  }, prefs);
  await mocked.goto('http://127.0.0.1:1436');
  await mocked.locator('.dock-item').filter({ hasText: '' }).first().waitFor();
  await mocked.getByRole('button', { name: 'Native fixture 0', exact: true }).click();
  assert.equal(await mocked.locator('html').getAttribute('data-chrome-theme'), 'dark');
  assert.equal(
    await mocked
      .locator('.app-shell')
      .evaluate((element) => getComputedStyle(element).backgroundColor),
    'rgb(25, 26, 32)'
  );
  await mocked.screenshot({ path: 'output/bobola-ui/dark-service-dock.png' }).catch(() => {});
  await mocked.getByRole('button', { name: 'Native fixture 1', exact: true }).click();
  assert.equal(await mocked.locator('html').getAttribute('data-chrome-theme'), 'light');
  assert.equal(
    await mocked
      .locator('.app-shell')
      .evaluate((element) => getComputedStyle(element).backgroundColor),
    'rgb(245, 245, 245)'
  );
  await mocked.getByRole('button', { name: 'Workspace settings', exact: true }).click();
  assert.ok((await mocked.locator('.protection-summary').innerText()).includes('23'));
  assert.equal(await mocked.getByRole('switch').count(), 0, 'account controls live on subpage');
  await mkdir('output/bobola-ui', { recursive: true });
  await mocked.screenshot({ path: 'output/bobola-ui/settings-overview.png' });
  await mocked.getByRole('button', { name: /Ad & tracker protection Account controls/ }).click();
  const nativeProtection = mocked.getByRole('region', { name: 'Ad and tracker protection' });
  assert.ok((await nativeProtection.innerText()).includes('23 network requests blocked'));
  const toggle = nativeProtection.getByRole('switch', {
    name: 'Protection for Native fixture 0',
    exact: true,
  });
  await toggle.click();
  assert.equal(await toggle.getAttribute('aria-checked'), 'false');
  assert.ok((await nativeProtection.innerText()).includes('23 network requests blocked'));
  await mocked.close();
  // A queued close event from the old dialog must not cancel a new native capture request.
  const delayed = await browser.newPage();
  await delayed.addInitScript(() => {
    window.__TAURI_INTERNALS__ = {
      invoke: async (name) => {
        if (name === 'bootstrap')
          return {
            version: 1,
            native: true,
            preferences: { liveBudget: 5, idleMinutes: 15, theme: 'light' },
            workspaces: [{ id: 'personal', name: 'Personal' }],
            interfacePreferences: { navMode: 'pinned', companion: { enabled: false } },
            services: [
              {
                id: 'race',
                name: 'Race fixture',
                url: 'https://example.com',
                workspaceId: 'personal',
                profileName: 'account_race',
                keepLive: true,
                protection: true,
              },
            ],
            runtime: {},
          };
        if (name === 'set_surface_layout') await new Promise((resolve) => setTimeout(resolve, 100));
        return null;
      },
    };
  });
  await delayed.goto('http://127.0.0.1:1436');
  await delayed.getByRole('button', { name: 'Manage Race fixture', exact: true }).click();
  await delayed.getByRole('dialog').waitFor();
  await delayed.evaluate(() => {
    document.querySelector('button[aria-label="Close dialog"]').click();
    document.querySelector('button[aria-label="Manage Race fixture"]').click();
  });
  await delayed.waitForTimeout(700);
  assert.equal(
    await delayed.getByRole('dialog').count(),
    1,
    'Manage reopens while the old close event is queued'
  );
  await delayed.close();
  await page.getByRole('button', { name: 'Workspace settings', exact: true }).click();
  await page.getByRole('button', { name: /Appearance & navigation/ }).click();
  await page.getByLabel('Service Navigation', { exact: true }).selectOption('minimal');
  await page.getByRole('button', { name: 'Save changes' }).click();
  assert.equal(await page.locator('.nav').evaluate((nav) => nav.getBoundingClientRect().width), 40);
  await page.reload();
  assert.equal(await page.locator('.nav').evaluate((nav) => nav.getBoundingClientRect().width), 40);
  await page.evaluate(() => {
    localStorage.setItem(
      'service-workspace-preview-v1',
      JSON.stringify({
        version: 1,
        preferences: { liveBudget: 5, idleMinutes: 15, theme: 'light' },
        workspaces: [{ id: 'personal', name: 'Personal' }],
        services: [
          {
            id: 'remove-me',
            name: 'Unopened fixture',
            url: 'https://example.com',
            workspaceId: 'personal',
            profileName: 'account_remove-me',
            keepLive: true,
            protection: true,
            color: 'blue',
          },
        ],
      })
    );
    localStorage.setItem(
      'service-workspace-preview-v1-interface',
      JSON.stringify({
        ...JSON.parse(localStorage.getItem('service-workspace-preview-v1-interface')),
        navMode: 'pinned',
        companion: { enabled: false, size: 40, motion: 'system' },
      })
    );
  });
  await page.reload();
  await page.getByRole('button', { name: 'Manage Unopened fixture', exact: true }).click();
  await page.getByRole('button', { name: 'Remove service', exact: true }).click();
  assert.ok(
    (await page.getByRole('dialog').innerText()).includes(
      'browser profile and sign-in data will remain'
    )
  );
  await page.getByRole('button', { name: 'Remove service', exact: true }).click();
  assert.equal(
    await page.getByRole('button', { name: 'Unopened fixture', exact: true }).count(),
    0
  );
  assert.equal(
    await page.evaluate(
      () => JSON.parse(localStorage.getItem('service-workspace-preview-v1')).services.length
    ),
    0
  );
  await page.keyboard.press('Control+k');
  await page.getByRole('dialog').getByRole('button', { name: 'Add service', exact: true }).click();
  assert.equal(await page.locator('.catalog > button').count(), 30);
  assert.equal(await page.locator('.catalog img').count(), 30);
  const artwork = await page.locator('.catalog img').evaluateAll(async (images) => {
    await Promise.all(images.map((image) => image.decode()));
    return images.every(
      (image) =>
        image.complete &&
        image.naturalWidth > 0 &&
        new URL(image.src).pathname.startsWith('/service-icons/')
    );
  });
  assert.ok(artwork, 'all thirty logos load from bundled local assets');
  const broadSearch = 'a';
  await page.getByLabel('Find an app').fill(broadSearch);
  assert.equal(
    await page.locator('.catalog > button').count(),
    fullCatalog.filter((entry) => entry.name.toLowerCase().includes(broadSearch)).length,
    'Search returns the full matching catalog'
  );
  await page.getByLabel('Find an app').fill('WhatsApp');
  await page.locator('.catalog').getByRole('button', { name: 'WhatsApp', exact: true }).click();
  assert.equal(await page.getByLabel('Background activity').inputValue(), 'true');
  await page.getByRole('button', { name: 'Custom service', exact: true }).click();
  await page.getByLabel('Service name', { exact: true }).fill('My music portal');
  await page.getByLabel('Website URL').fill('https://music.example.com');
  assert.equal(await page.getByLabel('Background activity').inputValue(), 'false');
  await page.getByRole('dialog').getByRole('button', { name: 'Add service', exact: true }).click();
  await page.locator('.preview-placeholder').waitFor();
  const custom = await page.evaluate(
    () => JSON.parse(localStorage.getItem('service-workspace-preview-v1')).services[0]
  );
  assert.equal(custom.keepLive, false);
  assert.equal(custom.catalogId, null);
  assert.equal(
    await page.locator('.nav img[src="/service-icons/youtubemusic.svg"]').count(),
    0,
    'custom name containing music must not get a YouTube Music logo'
  );
  await page.keyboard.press('Control+k');
  await page
    .getByRole('dialog')
    .getByRole('button', { name: 'Manage My music portal', exact: true })
    .click();
  await page.getByLabel('Account idle minutes').fill('3');
  await page.getByRole('button', { name: 'Apply', exact: true }).click();
  assert.equal(
    await page.evaluate(
      () => JSON.parse(localStorage.getItem('service-workspace-preview-v1')).services[0].idleMinutes
    ),
    3
  );
  await page.getByRole('button', { name: 'Close dialog', exact: true }).click();
  await page.keyboard.press('Control+k');
  await page.getByRole('button', { name: 'Memory & running accounts', exact: true }).click();
  await page.getByRole('region', { name: 'Resource usage', exact: true }).waitFor();
  assert.ok(
    (await page.getByRole('dialog').innerText()).includes(
      'Live process measurements are available in the Windows app.'
    )
  );
  await page.getByRole('button', { name: 'Close dialog', exact: true }).click();
  for (const mode of ['autohide', 'glass-dock', 'orb-launcher', 'minimal']) {
    await page.evaluate((mode) => {
      const key = 'service-workspace-preview-v1-interface';
      localStorage.setItem(
        key,
        JSON.stringify({ ...JSON.parse(localStorage.getItem(key)), navMode: mode })
      );
      const dataKey = 'service-workspace-preview-v1';
      const data = JSON.parse(localStorage.getItem(dataKey));
      data.services = Array.from({ length: 12 }, (_, index) => ({
        ...data.services[0],
        id: `unopened-${index}`,
        name: `Unopened ${index}`,
        profileName: `account_unopened-${index}`,
      }));
      localStorage.setItem(dataKey, JSON.stringify(data));
    }, mode);
    await page.reload();
    await page.keyboard.press('Control+k');
    await page.getByRole('dialog').getByLabel('Find a service').fill('Unopened 11');
    await page
      .getByRole('dialog')
      .getByRole('button', { name: 'Manage Unopened 11', exact: true })
      .click();
    await page.getByRole('button', { name: 'Remove service', exact: true }).click();
    await page.getByRole('button', { name: 'Remove service', exact: true }).click();
    assert.equal(
      await page.evaluate(
        () => JSON.parse(localStorage.getItem('service-workspace-preview-v1')).services.length
      ),
      11,
      `${mode} can remove an unopened account beyond the first eight icons`
    );
  }
  console.log(
    'PASS: 30 bundled service logos, catalog live defaults, custom-service sleep default, precise icon matching, quick-switch management, persisted per-account timeout and resource preview.'
  );
  console.log(
    'PASS: left dock, eight-icon limit, overflow selection, settings fit, mode persistence, non-overlapping orb arc, stable hover, switching, delayed collapse and keyboard expansion.'
  );
  console.log(
    'PASS: Bobola SVG, persisted 40px rail, right-side orb, settings subpages, dark theme, reduced motion, preview limitations, runtime-count aggregation and per-account protection command wiring (mock native fixture).'
  );
  console.log(
    'PASS: unopened services can be managed and removed through an in-app confirmation while browser profile retention is stated.'
  );
} finally {
  await browser?.close();
  server.kill();
}
