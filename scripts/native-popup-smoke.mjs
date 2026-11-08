// Native popup + overlay fixture. Windows only, debug build only. Not run by the worker.
// Usage: node scripts/native-popup-smoke.mjs [exe]
// Build first: node scripts/rust.mjs build -p service-workspace
// A failed scenario writes output/native/popup-failure.json and keeps its sw-popup-* directory.
import http from 'node:http';
import { spawn } from 'node:child_process';
import {
  existsSync,
  lstatSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  statSync,
  writeFileSync,
} from 'node:fs';
import { rm } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import assert from 'node:assert/strict';

// Same resolution as scripts/rust.mjs and native-smoke.mjs: argument, then CARGO_TARGET_DIR, then
// <tmp>/service-workspace-target. SERVICE_WORKSPACE_EXE stays as an explicit override.
const exe = path.resolve(
  process.argv[2] ??
    process.env.SERVICE_WORKSPACE_EXE ??
    path.join(
      process.env.CARGO_TARGET_DIR ?? path.join(os.tmpdir(), 'service-workspace-target'),
      'debug',
      'service-workspace.exe'
    )
);
const failureFile = path.resolve('output', 'native', 'popup-failure.json');
const reported = new WeakSet();
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const readText = (file) => {
  try {
    return readFileSync(file, 'utf8');
  } catch {
    return null;
  }
};
// The app rewrites its snapshot files every 500 ms, so a read can see a partial file. That yields null.
const readJson = (file) => {
  try {
    return JSON.parse(readFileSync(file, 'utf8'));
  } catch {
    return null;
  }
};
const tail = (text, max = 4000) =>
  text.length > max ? `[${text.length - max} chars cut]${text.slice(-max)}` : text;
// Caps the serialized size of a value so the failure file stays small.
function bound(value, max = 20000) {
  let text;
  try {
    text = JSON.stringify(value);
  } catch {
    text = String(value);
  }
  if (text === undefined || text.length <= max) return value;
  return { truncated: true, length: text.length, head: text.slice(0, max) };
}
const lastEntries = (list, max = 200) => ({
  total: list.length,
  entries: list.slice(-max).map((entry) => bound(entry, 4000)),
});
// Resolves true if the promise settled first and false on timeout. The timer never keeps node alive.
async function within(promise, ms) {
  let timer;
  const timeout = new Promise((resolve) => {
    timer = setTimeout(() => resolve(false), ms);
    timer.unref?.();
  });
  try {
    return await Promise.race([promise.then(() => true), timeout]);
  } finally {
    clearTimeout(timer);
  }
}

// Directories this process created. Nothing outside this set is ever deleted.
const owned = new Set();
// A path is deletable only if this process created it, it sits directly inside os.tmpdir(), its name
// starts with sw-popup-, and it is not a symlink.
function isOwnedTemp(target) {
  const resolved = path.resolve(target);
  const root = path.resolve(os.tmpdir());
  const same = (a, b) => (process.platform === 'win32' ? a.toLowerCase() === b.toLowerCase() : a === b);
  if (!path.isAbsolute(resolved) || !same(path.dirname(resolved), root)) return false;
  if (!path.basename(resolved).startsWith('sw-popup-')) return false;
  if (![...owned].some((entry) => same(entry, resolved))) return false;
  try {
    return !lstatSync(resolved).isSymbolicLink();
  } catch (error) {
    return error?.code === 'ENOENT';
  }
}
// Retries because WebView2 can hold files in the directory for a moment after the app exits.
// Returns a warning string on failure and never throws.
async function removeOwned(target, budgetMs = 20000) {
  if (!isOwnedTemp(target)) return `Refused to delete ${target}: not an owned sw-popup-* directory in the temp folder`;
  const end = Date.now() + budgetMs;
  let last;
  for (let attempt = 0; ; attempt++) {
    try {
      await rm(target, { recursive: true, force: true });
      return null;
    } catch (error) {
      last = error;
      if (Date.now() >= end) break;
      await sleep(Math.min(250 * (attempt + 1), 2000));
    }
  }
  return `Could not delete ${target} (${last?.code ?? last?.message}); remove it manually`;
}
// server.close() waits for keep-alive sockets, so drop them first and bound the wait.
async function closeServer(server) {
  const closed = new Promise((resolve) => server.close(() => resolve()));
  server.closeIdleConnections?.();
  server.closeAllConnections?.();
  await within(closed, 3000);
}

// Runs inside each scenario's try block so a failure here is reported like any other.
function preflight() {
  // These make WebView2 ignore the isolated profile. The app also refuses to start with them.
  for (const name of [
    'WEBVIEW2_USER_DATA_FOLDER',
    'WEBVIEW2_BROWSER_EXECUTABLE_FOLDER',
    'WEBVIEW2_RELEASE_CHANNEL_PREFERENCE',
    'WEBVIEW2_CHANNEL_SEARCH_KIND',
    'TAURI_CONFIG',
  ]) {
    assert.equal(process.env[name], undefined, `${name} must not be set for the native popup test`);
  }
  let stats;
  try {
    stats = statSync(exe);
  } catch (error) {
    throw new Error(
      `Executable not found: ${exe} (${error.code}). Build it with: node scripts/rust.mjs build -p service-workspace`
    );
  }
  assert.ok(stats.isFile(), `Executable path is not a file: ${exe}`);
}

function describeExit(run) {
  const { child } = run;
  if (run.spawnError) return `failed to start: ${run.spawnError.code ?? ''} ${run.spawnError.message}`;
  if (child.exitCode !== null) return `exited with code ${child.exitCode}`;
  if (child.signalCode) return `was killed by ${child.signalCode}`;
  return 'is still running';
}

function cookies(header = '') {
  return Object.fromEntries(
    header
      .split(';')
      .map((part) => part.trim().split('='))
      .filter(([name]) => name)
  );
}

function fixtureServer() {
  const log = [];
  const requests = [];
  const page = (role) => `<!doctype html><meta charset=utf-8><title>Fixture ${role}</title><body>
<script>
const role = ${JSON.stringify(role)};
const post = (kind, data = {}) => fetch('/log', { method: 'POST', body: JSON.stringify({ role, kind, ...data }) });
const messages = [];
addEventListener('message', (event) => { if (event.data && event.data.popup) messages.push(event.data); });
addEventListener('load', async () => {
  // Main view reports unsaved work; popup activity must never clear it.
  setTimeout(() => document.body.dispatchEvent(new Event('input', { bubbles: true })), 300);
  await new Promise((r) => setTimeout(r, 1200));
  const direct = window.open('/popup?role=' + role);
  await post('direct-open', { opened: direct !== null });
  await new Promise((r) => setTimeout(r, 2500));
  await post('opener-messages', { messages, cookie: document.cookie });
  const redirected = window.open('/redirect?role=' + role);
  await post('redirect-open', { opened: redirected !== null });
  await new Promise((r) => setTimeout(r, 2500));
  // about:blank is permitted per spec/policy; file:// and custom schemes are blocked.
  const blocked = ['about:blank', 'file:///C:/Windows/win.ini', 'service-workspace-test://x'].map((u) => window.open(u) === null);
  await post('blocked', { blocked });
  let opened = 0;
  for (let i = 0; i < 20; i++) if (window.open('/hold?role=' + role) !== null) opened++;
  await post('flood', { opened });
});
</script>`;
  const server = http.createServer((req, res) => {
    const url = new URL(req.url, 'http://127.0.0.1');
    // The role is interpolated into pages and a Location header, so keep only word characters.
    const rawRole = url.searchParams.get('role');
    const role = rawRole === null ? null : rawRole.replace(/[^\w-]/g, '');
    requests.push({ path: url.pathname, role, cookie: cookies(req.headers.cookie) });
    if (url.pathname === '/log') {
      let body = '';
      req.on('data', (c) => (body += c));
      req.on('error', () => res.destroy());
      req.on('end', () => {
        // A malformed body must not throw out of the handler and hide the real failure.
        try {
          log.push(JSON.parse(body));
          res.end('ok');
        } catch (error) {
          log.push({ kind: 'fixture-bad-log', error: String(error), body: body.slice(0, 500) });
          res.writeHead(400);
          res.end('bad');
        }
      });
      return;
    }
    if (url.pathname === '/redirect') {
      res.writeHead(302, { Location: `/popup?role=${role}&via=redirect` });
      return res.end();
    }
    const headers = { 'Content-Type': 'text/html' };
    if (url.pathname === '/fixture') headers['Set-Cookie'] = `acct=${role}; Path=/`;
    res.writeHead(200, headers);
    if (url.pathname === '/popup') {
      res.end(`<!doctype html><script>
        document.cookie = 'popupmark=${role}; Path=/';
        const bridge = ['__TAURI__', '__TAURI_INTERNALS__', 'ipc'].some((n) => n in window);
        fetch('/log', { method: 'POST', body: JSON.stringify({ role: '${role}', kind: 'popup-loaded', bridge, hasOpener: !!window.opener, cookie: document.cookie }) });
        window.opener && window.opener.postMessage({ popup: true, role: '${role}', cookie: document.cookie }, '*');
        setTimeout(() => window.close(), 400);
      </script>`);
    } else if (url.pathname === '/hold') {
      // The script must come after <body> opens. Without it, document.body is null here and the
      // dispatchEvent line throws after the log request has already been sent.
      res.end(`<!doctype html><title>hold</title><body><script>
        fetch('/log', { method: 'POST', body: JSON.stringify({ role: '${role}', kind: 'hold-loaded' }) });
        document.body.dispatchEvent(new Event('input', { bubbles: true }));
      </script>`);
    } else {
      res.end(page(role));
    }
  });
  return new Promise((resolve) =>
    server.listen(0, '127.0.0.1', () => resolve({ server, log, requests, port: server.address().port }))
  );
}

async function launch(name, extra) {
  const fixture = await fixtureServer();
  let dir;
  let child;
  try {
    dir = mkdtempSync(path.join(os.tmpdir(), 'sw-popup-'));
    owned.add(path.resolve(dir));
    child = spawn(exe, [], {
      env: {
        ...process.env,
        SERVICE_WORKSPACE_TEST_DIR: dir,
        SERVICE_WORKSPACE_TEST_ORIGIN: `http://127.0.0.1:${fixture.port}`,
        SERVICE_WORKSPACE_TEST_PHASE: 'first',
        ...extra,
      },
      stdio: ['ignore', 'ignore', 'pipe'],
    });
  } catch (error) {
    // Nothing is running yet, so the empty directory and the server can go.
    await closeServer(fixture.server);
    if (dir) await removeOwned(dir, 3000);
    throw error;
  }
  const run = { name, fixture, dir, child, step: 'launch', exited: false, spawnError: null, stderr: '', keep: false };
  attach(run);
  return run;
}

// Listeners are attached in the same tick as spawn, so no exit or error event can be missed.
function attach(run) {
  const { child } = run;
  run.done = new Promise((resolve) => {
    child.on('exit', () => {
      run.exited = true;
      resolve();
    });
    // A failed spawn emits 'error' and never 'exit'. Later errors (a failed kill) keep the run alive.
    child.on('error', (error) => {
      run.spawnError ??= error;
      if (child.pid === undefined) {
        run.exited = true;
        resolve();
      }
    });
  });
  child.stderr?.on('data', (data) => {
    run.stderr = (run.stderr + data).slice(-20000);
  });
  child.stderr?.on('error', () => {});
  run.alive = () => !run.exited && child.exitCode === null && child.signalCode === null;
  run.mark = (step) => {
    run.step = step;
  };
  run.stop = () => stopRun(run);
  // Polls a synchronous check. If the app dies first, the error names the exit and native-error.txt.
  run.waitFor = async (check, what, ms = 30000) => {
    const end = Date.now() + ms;
    for (;;) {
      const value = check();
      if (value) return value;
      if (!run.alive()) {
        const last = check();
        if (last) return last;
        const nativeError = readText(path.join(run.dir, 'native-error.txt'));
        throw new Error(
          `Native app ${describeExit(run)} while waiting for ${what}` +
            (nativeError ? `\nnative-error.txt: ${tail(nativeError)}` : '') +
            (run.stderr ? `\nstderr: ${tail(run.stderr, 2000)}` : '')
        );
      }
      if (Date.now() > end) throw new Error(`Timed out waiting for ${what}`);
      await sleep(200);
    }
  };
}

// Stops only the app this run spawned. It never throws, so it cannot replace the original failure.
// With run.keep set, the test directory is left in place.
async function stopRun(run) {
  const { child } = run;
  try {
    if (run.alive()) {
      try {
        writeFileSync(path.join(run.dir, 'exit-request'), '1');
      } catch {}
      if (!(await within(run.done, 10000))) {
        try {
          child.kill();
        } catch {}
        if (!(await within(run.done, 5000))) console.warn(`Warning: app pid ${child.pid} did not exit after kill`);
      }
    }
    await closeServer(run.fixture.server);
    if (run.keep) {
      console.warn(`Kept test directory for inspection: ${run.dir}`);
    } else {
      // WebView2 helper processes exit on their own once the host is gone and can hold files briefly.
      const warning = await removeOwned(run.dir);
      if (warning) console.warn(`Warning: ${warning}`);
    }
  } catch (error) {
    console.warn(`Warning: cleanup failed: ${error?.message ?? error}`);
  } finally {
    child.unref?.();
    child.stderr?.destroy();
  }
}

function errorInfo(error) {
  if (!(error instanceof Error)) return { message: String(error) };
  const info = { name: error.name, message: error.message, stack: error.stack };
  for (const key of ['code', 'operator', 'actual', 'expected']) {
    if (key in error) info[key] = bound(error[key], 2000);
  }
  return bound(info, 10000);
}

// Reads a file the app may be rewriting. Retries a partial JSON read, then falls back to bounded text.
async function snapshotFile(file) {
  for (let attempt = 0; attempt < 3; attempt++) {
    const text = readText(file);
    if (text === null) return null;
    try {
      return bound(JSON.parse(text));
    } catch {
      if (attempt < 2) await sleep(150);
      else return { unparsed: tail(text, 4000) };
    }
  }
  return null;
}

// Runs before cleanup. Never throws, so the original error is always printed by the caller.
async function captureFailure(scenario, run, error) {
  const report = {
    status: 'failed',
    scenario,
    time: new Date().toISOString(),
    executable: exe,
    error: errorInfo(error),
  };
  if (run) {
    const { dir } = run;
    Object.assign(report, {
      step: run.step,
      testDirectory: dir,
      testDirectoryRetained: true,
      app: {
        pid: run.child.pid ?? null,
        status: describeExit(run),
        exitCode: run.child.exitCode,
        signal: run.child.signalCode,
        spawnError: run.spawnError ? { code: run.spawnError.code, message: run.spawnError.message } : null,
      },
      directoryListing: (() => {
        try {
          return readdirSync(dir).slice(0, 100);
        } catch (listError) {
          return `unreadable: ${listError.code ?? listError.message}`;
        }
      })(),
      stderrTail: tail(run.stderr, 4000),
      fixtureLog: lastEntries(run.fixture.log),
      fixtureRequests: lastEntries(run.fixture.requests),
      nativeError: (() => {
        const text = readText(path.join(dir, 'native-error.txt'));
        return text === null ? null : tail(text, 4000);
      })(),
      nativePopups: await snapshotFile(path.join(dir, 'native-popups.json')),
      nativeRuntime: await snapshotFile(path.join(dir, 'native-runtime.json')),
      nativeOverlay: await snapshotFile(path.join(dir, 'native-overlay.json')),
    });
  }
  try {
    mkdirSync(path.dirname(failureFile), { recursive: true });
    writeFileSync(failureFile, JSON.stringify(report, null, 2));
    console.error(`Failure details written to ${failureFile}`);
  } catch (writeError) {
    console.warn(`Warning: could not write ${failureFile}: ${writeError.message}`);
  }
}

// Order on failure: print the original error, write popup-failure.json, then stop and clean up.
// Cleanup problems are warnings and cannot replace the assertion.
async function scenario(name, extra, body) {
  let run;
  try {
    preflight();
    run = await launch(name, extra);
    await body(run);
    console.log(`${name} scenario passed`);
  } catch (error) {
    console.error(`${name} scenario FAILED:`);
    console.error(error);
    if (error !== null && typeof error === 'object') reported.add(error);
    if (run) run.keep = true;
    await captureFailure(name, run, error);
    throw error;
  } finally {
    if (run) await run.stop();
  }
}

async function popupScenario(run) {
  const { fixture, dir } = run;
  try {
    const popupsFile = path.join(dir, 'native-popups.json');
    // The app rewrites this file every 500 ms. A partial read is retried instead of failing the check.
  const readCount = async (name) =>
    (
      await run.waitFor(() => {
        const count = readJson(popupsFile)?.[name]?.count;
        return typeof count === 'number' ? { count } : null;
      }, `${name} popup count`)
    ).count;
  run.mark('wait for both accounts to finish');
  await run.waitFor(() => fixture.log.filter((e) => e.kind === 'flood').length === 2, 'both accounts to finish');
  assert.ok(!existsSync(path.join(dir, 'native-error.txt')), 'native host reported an error');
  run.mark('fixture log assertions');
  for (const role of ['A', 'B']) {
    const mine = fixture.log.filter((e) => e.role === role);
    const by = (kind) => {
      const entry = mine.find((e) => e.kind === kind);
      assert.ok(entry, `${role}: fixture never logged ${kind}`);
      return entry;
    };
    assert.equal(by('direct-open').opened, true, `${role}: direct popup opened`);
    assert.equal(by('redirect-open').opened, true, `${role}: redirected popup opened`);
    const loaded = mine.filter((e) => e.kind === 'popup-loaded');
    assert.equal(loaded.length, 2, `${role}: direct and redirected popup loaded`);
    assert.ok(loaded.every((e) => e.bridge === false), `${role}: no Tauri bridge in popup`);
    assert.ok(loaded.every((e) => e.hasOpener), `${role}: window.opener available`);
    assert.ok(by('opener-messages').messages.some((m) => m.role === role), `${role}: opener received postMessage`);
    // Same-account cookie sharing: opener sees the popup's cookie, popup saw the account cookie.
    assert.match(by('opener-messages').cookie, new RegExp(`popupmark=${role}`));
    assert.ok(loaded.every((e) => e.cookie.includes(`acct=${role}`)), `${role}: popup shares account cookie`);
    assert.deepEqual(by('blocked').blocked, [false, true, true], `${role}: unsafe schemes refused`);
    const flood = by('flood').opened;
    assert.ok(flood >= 1 && flood <= 4, `${role}: flood limited (opened ${flood})`);
  }
  // PLACEHOLDER_POPUP_TAIL
    // Account separation: no request ever carries the other account's cookies.
    for (const r of fixture.requests.filter((q) => q.role && ['/fixture', '/popup', '/hold', '/redirect'].includes(q.path))) {
      if (r.cookie.acct !== undefined) assert.equal(r.cookie.acct, r.role, `cookie leak on ${r.path}`);
      if (r.cookie.popupmark !== undefined) assert.equal(r.cookie.popupmark, r.role, `popup cookie leak on ${r.path}`);
    }
    // window.close() cleaned up the first two popups; only held ones remain.
    const snapshot = await run.waitFor(() => {
      const value = readJson(path.join(dir, 'native-popups.json'));
      return value && value['Fixture A'].count >= 1 && value['Fixture B'].count >= 1 ? value : null;
    }, 'popup snapshot');
    for (const name of ['Fixture A', 'Fixture B']) assert.ok(snapshot[name].count <= 4);
    // Popup activity is tracked apart from the main view: main dirty flags stay set.
    const runtime = readJson(path.join(dir, 'native-runtime.json'));
    assert.ok(Object.values(runtime).every((r) => r.dirty), 'main-view dirty flag survived popup activity');
    // Automatic/user unload refuses an account that has popups.
    writeFileSync(path.join(dir, 'unload-A'), '1');
    const refusal = await run.waitFor(() => existsSync(path.join(dir, 'unload-A.result')) && readFileSync(path.join(dir, 'unload-A.result'), 'utf8'), 'unload result');
    assert.match(refusal, /popup/i);
    // Removing account A closes its popups and leaves B's alone.
    const bBefore = readJson(path.join(dir, 'native-popups.json'))['Fixture B'].count;
    writeFileSync(path.join(dir, 'remove-A'), '1');
    await run.waitFor(() => existsSync(path.join(dir, 'remove-A.result')), 'remove result');
    assert.equal(readFileSync(path.join(dir, 'remove-A.result'), 'utf8'), 'ok');
    await run.waitFor(() => readJson(path.join(dir, 'native-popups.json'))['Fixture A'].count === 0, 'A popups closed');
    assert.equal(readJson(path.join(dir, 'native-popups.json'))['Fixture B'].count, bBefore);
    writeFileSync(path.join(dir, 'remove-B'), '1');
    await run.waitFor(() => readJson(path.join(dir, 'native-popups.json'))['Fixture B'].count === 0, 'B popups closed');
    console.log('popup scenario passed');
  } finally {
    await run.stop();
  }
}

async function overlayScenario() {
  await scenario('overlay', { SERVICE_WORKSPACE_TEST_OVERLAY: '1' }, async (run) => {
    const report = await run.waitFor(() => readJson(path.join(run.dir, 'native-overlay.json')), 'overlay report', 45000);
    assert.ok(!existsSync(path.join(run.dir, 'native-error.txt')), 'native host reported an error');
    const hits = Object.fromEntries(report.map((s) => [s.step, s.probe.hits]));
    assert.deepEqual(hits['hole'], [false, true], 'exclusion lets shell through, rest stays on the site');
    assert.deepEqual(hits['no-hole'], [true, true]);
    assert.deepEqual(hits['modal-capture'], [false, false], 'modal capture removes site input');
    assert.deepEqual(hits['capture-released'], [true, true], 'input restored after capture');
    assert.deepEqual(hits['stale-ignored'], [true, true], 'stale revision is ignored');
    const sizes = new Set(report.map((s) => `${s.probe.width}x${s.probe.height}`));
    assert.equal(sizes.size, 1, 'website viewport size never changed');
    console.log('overlay scenario passed');
  });
}

await scenario('popup', undefined, async (run) => {
  await popupScenario(run);
});
await overlayScenario();
