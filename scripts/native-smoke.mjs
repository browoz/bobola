import http from 'node:http';
import { spawn } from 'node:child_process';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import assert from 'node:assert/strict';
import { chromium } from 'playwright';

const executable =
  process.argv[2] ??
  path.join(os.tmpdir(), 'service-workspace-target', 'debug', 'service-workspace.exe');
// The owned fixture also writes native runtime snapshots, allowing lifecycle and
// interception checks on machines where the WebView2 CDP endpoint is unavailable.
const reportsOnly = process.argv.includes('--reports-only');
const directory = path.resolve('.local', `native-smoke-${Date.now()}`);
await fs.mkdir(directory, { recursive: true });
await fs.mkdir('output/native', { recursive: true });
const reports = [];
let child;
let stderr = '';
let navigateClean = false;
const styleReports = [];
const server = http.createServer(async (request, response) => {
  const url = new URL(request.url, 'http://127.0.0.1');
  if (url.pathname === '/control') {
    response.writeHead(200, { 'Content-Type': 'application/json', 'Cache-Control': 'no-store' });
    response.end(
      JSON.stringify({ navigate: navigateClean && url.searchParams.get('role') === 'A' })
    );
    return;
  }
  if (url.pathname === '/style-report') {
    styleReports.push({
      role: url.searchParams.get('role'),
      hidden: url.searchParams.get('hidden') === 'true',
    });
    response.writeHead(204).end();
    return;
  }
  if (url.pathname === '/report') {
    let body = '';
    for await (const chunk of request) body += chunk;
    reports.push(JSON.parse(body));
    response.writeHead(204);
    response.end();
    return;
  }
  if (url.pathname === '/workspace-blocked.js') {
    response.writeHead(200, { 'Content-Type': 'application/javascript' });
    response.end('window.fixtureAdLoaded=true');
    return;
  }
  if (url.pathname !== '/fixture') {
    response.writeHead(404);
    response.end();
    return;
  }
  const role = url.searchParams.get('role'),
    phase = url.searchParams.get('phase');
  const safeRole = role === 'A' || role === 'B' ? role : 'A';
  const safePhase = phase === 'first' || phase === 'restart' ? phase : 'first';
  response.writeHead(200, { 'Content-Type': 'text/html', 'Cache-Control': 'no-store' });
  response.end(`<!doctype html><html><head><title>Fixture ${safeRole}</title><style>html,body{background:${safeRole === 'A' ? '#191a20' : '#f5f5f5'};min-height:100vh}</style></head><body><div class="sponsored">Fixture advertisement</div><input id="draft" aria-label="Draft"><script>
  const role=${JSON.stringify(safeRole)},phase=${JSON.stringify(safePhase)};
  const before={cookie:document.cookie,storage:localStorage.getItem('fixture-account')};
  document.cookie='fixture-account='+role+'; Path=/; Max-Age=3600; SameSite=Lax';localStorage.setItem('fixture-account',role);
  const ad=document.createElement('script');ad.src='/workspace-blocked.js';document.head.append(ad);
  if(!new URL(location.href).searchParams.has('clean'))setTimeout(()=>{const input=document.getElementById('draft');input.value='unsaved fixture';input.dispatchEvent(new Event('input',{bubbles:true}));},200);
  setInterval(async()=>{fetch('/style-report?role='+role+'&hidden='+(getComputedStyle(document.querySelector('.sponsored')).display==='none'));if(!new URL(location.href).searchParams.has('clean')){const control=await fetch('/control?role='+role).then(response=>response.json());if(control.navigate)location.href=location.pathname+'?role='+role+'&phase='+phase+'&clean=1#remembered-route';}},500);
  setTimeout(()=>fetch('/report',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({role,phase,before,href:location.href,cookie:document.cookie,storage:localStorage.getItem('fixture-account'),hasTauriBridge:'__TAURI_INTERNALS__' in window,adLoaded:!!window.fixtureAdLoaded,cosmeticHidden:getComputedStyle(document.querySelector('.sponsored')).display==='none'})}),2500);
  </script></body></html>`);
});
await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
const origin = `http://127.0.0.1:${server.address().port}`;
const delay = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
async function waitUntil(check, message, timeout = 60000) {
  const start = Date.now();
  while (Date.now() - start < timeout) {
    if (await check()) return;
    await delay(250);
  }
  throw new Error(`${message}\n${stderr.slice(-3000)}`);
}
async function phase(name) {
  await fs.rm(path.join(directory, 'exit-request'), { force: true });
  const portServer = http.createServer();
  await new Promise((resolve) => portServer.listen(0, '127.0.0.1', resolve));
  const debugPort = portServer.address().port;
  await new Promise((resolve) => portServer.close(resolve));
  child = spawn(executable, [], {
    windowsHide: true,
    stdio: ['ignore', 'ignore', 'pipe'],
    env: {
      ...process.env,
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${debugPort}`,
      SERVICE_WORKSPACE_TEST_DIR: directory,
      SERVICE_WORKSPACE_TEST_ORIGIN: origin,
      SERVICE_WORKSPACE_TEST_PHASE: name,
    },
  });
  child.stderr.on('data', (data) => {
    stderr += data;
  });
  child.on('error', (error) => {
    stderr += error.message;
  });
  let browser, shellPage;
  if (!reportsOnly) {
    await waitUntil(async () => {
      try {
        return (await fetch(`http://127.0.0.1:${debugPort}/json/version`)).ok;
      } catch {
        return false;
      }
    }, 'Native shell inspection endpoint did not start');
    browser = await chromium.connectOverCDP(`http://127.0.0.1:${debugPort}`);
    await waitUntil(() => {
      shellPage = browser
        .contexts()
        .flatMap((context) => context.pages())
        .find(
          (page) => page.url().includes('tauri.localhost') || page.url().startsWith('tauri://')
        );
      return !!shellPage;
    }, 'Native shell page was not found');
    const snapshot = await shellPage.evaluate(() => window.__TAURI_INTERNALS__.invoke('bootstrap'));
    assert.equal(snapshot.native, true, 'The local shell should reach the native Rust command');
    assert.equal(snapshot.services.length, 2);
    const resources = await shellPage.evaluate(() =>
      window.__TAURI_INTERNALS__.invoke('resource_sample')
    );
    assert.ok(
      resources.processes.some((process) => process.pid === child.pid),
      'Resources include the owned app root'
    );
    assert.ok(
      resources.processes.filter((process) => process.name.toLowerCase() === 'msedgewebview2.exe')
        .length >= 2,
      'Resources include browser child processes'
    );
    assert.equal(
      new Set(resources.processes.map((process) => process.pid)).size,
      resources.processes.length,
      'Shared browser processes count once'
    );
    assert.ok(
      resources.processes.every(
        (process) =>
          process.privateBytes > 0 && process.workingSetBytes > 0 && process.cpuSeconds >= 0
      ),
      'Owned fixture process measurements are available'
    );
    await fs.writeFile(
      path.join(directory, `resources-${name}.json`),
      JSON.stringify(resources, null, 2)
    );
  }
  await waitUntil(
    () => reports.filter((report) => report.phase === name).length === 2,
    `Missing native reports for ${name}`
  );
  await delay(1500);
  const runtime = JSON.parse(
    await fs.readFile(path.join(directory, 'native-runtime.json'), 'utf8')
  );
  const views = Object.values(runtime);
  assert.equal(views.length, 2);
  assert.ok(views.every((view) => view.loaded));
  assert.ok(
    views.some((view) => JSON.stringify(view.background) === '[25,26,32]'),
    'Native page bridge reports dark service background'
  );
  assert.ok(
    views.some((view) => JSON.stringify(view.background) === '[245,245,245]'),
    'Native page bridge reports light service background'
  );
  assert.ok(views[0].browserPid > 0);
  assert.equal(
    views[0].browserPid,
    views[1].browserPid,
    'Account views should share one browser process'
  );
  assert.ok(
    views.every((view) => view.dirty),
    'Native page-state guards should record unsaved edits'
  );
  const phaseReports = reports.filter((report) => report.phase === name);
  for (const report of phaseReports) {
    const protectedAccount = name === 'first' || report.role === 'B';
    assert.equal(
      report.adLoaded,
      !protectedAccount,
      'Actual network interception should honor the per-service setting'
    );
    assert.equal(
      report.cosmeticHidden,
      protectedAccount,
      'Actual cosmetic injection should honor the per-service setting'
    );
    assert.equal(
      report.hasTauriBridge,
      false,
      'Remote service pages must not inherit the local shell native bridge'
    );
    assert.equal(report.storage, report.role);
    assert.ok(report.cookie.includes(`fixture-account=${report.role}`));
    if (name === 'first') {
      assert.equal(report.before.cookie, '');
      assert.equal(report.before.storage, null);
    } else {
      assert.ok(
        report.before.cookie.includes(`fixture-account=${report.role}`),
        'Persistent cookies should survive restart'
      );
      assert.equal(report.before.storage, report.role);
    }
  }
  assert.ok(
    views.some((view) => view.blocked > 0),
    'The real native interceptor should record blocking'
  );
  if (shellPage && name === 'first') {
    const service = await shellPage.evaluate(async () =>
      (await window.__TAURI_INTERNALS__.invoke('bootstrap')).services.find(
        (service) => service.name === 'Fixture A'
      )
    );
    const call = (command, args = {}) =>
      shellPage.evaluate(({ command, args }) => window.__TAURI_INTERNALS__.invoke(command, args), {
        command,
        args,
      });
    await assert.rejects(
      () => call('unload_service', { id: service.id }),
      /open|unsaved/i,
      'Visible dirty pages refuse sleep'
    );
    styleReports.length = 0;
    await shellPage.evaluate(
      (service) =>
        window.__TAURI_INTERNALS__.invoke('update_service', {
          service: { ...service, protection: false },
        }),
      service
    );
    await waitUntil(
      () => styleReports.some((report) => report.role === 'A' && !report.hidden),
      'Disabling protection did not immediately clear cosmetic CSS'
    );
    await call('set_layout', { layout: [] });
    await assert.rejects(
      () => call('unload_service', { id: service.id }),
      /unsaved/i,
      'Hidden dirty pages refuse sleep'
    );
    navigateClean = true;
    await waitUntil(
      async () => !(await call('bootstrap')).runtime[service.id]?.dirty,
      'Clean navigation did not reset dirty state'
    );
    await call('set_layout', { layout: [] });
    await call('unload_service', { id: service.id });
    const sleeping = (await call('bootstrap')).runtime[service.id];
    assert.equal(sleeping.loaded, false);
    assert.ok(
      sleeping.lastUrl.endsWith('&clean=1#remembered-route'),
      'Sleeping remembers the safe current URL'
    );
    const beforeWake = reports.length;
    await call('set_layout', {
      layout: [{ serviceId: service.id, x: 0, y: 100, width: 500, height: 600 }],
    });
    await waitUntil(
      () =>
        reports
          .slice(beforeWake)
          .some(
            (report) => report.role === 'A' && report.href.endsWith('&clean=1#remembered-route')
          ),
      'Wake did not resume the remembered URL'
    );
    const wake = reports.slice(beforeWake).find((report) => report.role === 'A');
    assert.equal(wake.before.storage, 'A');
    assert.ok(
      wake.before.cookie.includes('fixture-account=A'),
      'Sign-in cookies survive sleep and wake'
    );
    navigateClean = false;
    // Exercise Manage against real native layout commands after the lifecycle assertions.
    const prefs = (await call('bootstrap')).interfacePreferences;
    await call('save_interface_preferences', { preferences: { ...prefs, navMode: 'glass-dock' } });
    await shellPage.reload();
    const fixtureButton = shellPage.getByRole('button', { name: 'Fixture A', exact: true });
    await fixtureButton.waitFor();
    await fixtureButton.click();
    await fixtureButton.click({ button: 'right' });
    await shellPage
      .getByRole('group', { name: 'Fixture A actions' })
      .getByRole('button', { name: 'Manage', exact: true })
      .click();
    await shellPage.getByRole('switch', { name: 'Keep service live', exact: true }).waitFor();
    assert.ok(
      Object.values((await call('bootstrap')).runtime).every((view) => !view.visible),
      'Manage captures native website surfaces'
    );
    await shellPage.screenshot({ path: 'output/native/manage-dialog.png' });
    await shellPage.getByRole('button', { name: 'Close dialog', exact: true }).click();
    await waitUntil(
      async () => (await call('bootstrap')).runtime[service.id]?.visible,
      'Closing Manage did not restore the website'
    );
  }
  await fs.writeFile(path.join(directory, 'exit-request'), 'exit');
  await waitUntil(() => child.exitCode !== null, 'Native app did not exit cleanly', 20000);
  await browser?.close();
  return { reports: phaseReports, runtime };
}
try {
  const first = await phase('first');
  const restart = await phase('restart');
  const result = {
    status: 'passed',
    executable,
    checks: [
      ...(reportsOnly
        ? []
        : [
            'local-shell native IPC',
            'owned app and browser resource measurements with deduplicated PIDs',
            'immediate cosmetic removal on disable',
            'visible/dirty sleep refusal',
            'sleep/wake remembers URL and preserves cookies and local storage',
            'dock Manage dialog captures native views and restores them on close',
          ]),
      'remote pages have no Tauri bridge',
      'same-origin cookie separation',
      'same-origin local-storage separation',
      'shared browser process',
      'native network blocking',
      'main-document cosmetic filtering',
      'per-service protection disable',
      'persistent profiles after restart',
      'native dirty-state reporting',
      'native light and dark service-color reporting',
    ],
    shellIpcChecked: !reportsOnly,
    first,
    restart,
  };
  await fs.writeFile('output/native/smoke-result.json', JSON.stringify(result, null, 2));
  console.log(JSON.stringify({ status: result.status, checks: result.checks }));
} catch (error) {
  await fs.writeFile(
    'output/native/smoke-failure.json',
    JSON.stringify({ status: 'failed', error: error.message, reports, stderr }, null, 2)
  );
  console.error(error);
  process.exitCode = 1;
} finally {
  if (child && child.exitCode === null) child.kill();
  server.close();
}
