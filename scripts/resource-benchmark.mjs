import assert from 'node:assert/strict';
import { spawn, execFileSync } from 'node:child_process';
import fs from 'node:fs/promises';
import http from 'node:http';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  buildResourceSampler,
  validateResourceSamples,
  resourceCpu,
} from './resource-measurements.mjs';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const defaultTarget =
  process.env.CARGO_TARGET_DIR ?? path.join(os.tmpdir(), 'service-workspace-target');
const args = process.argv.slice(2);
const usesDefaultExecutable = args[0] === undefined || /^\d+$/.test(args[0]);
const executable = path.resolve(
  usesDefaultExecutable
    ? path.join(defaultTarget, 'release', 'service-workspace.exe')
    : args.shift()
);
const sampleSeconds = Number(args[0] ?? 20);
const runCount = Number(args[1] ?? 3);
const selectedCases = args[2]?.split(',').filter(Boolean) ?? null;
assert.ok(
  Number.isInteger(sampleSeconds) && sampleSeconds >= 10 && sampleSeconds <= 120,
  'Sample duration must be an integer from 10-120 seconds.'
);
assert.ok(
  Number.isInteger(runCount) && runCount >= 1 && runCount <= 10,
  'Run count must be an integer from 1-10.'
);
const executableStat = await fs.stat(executable).catch(() => null);
assert.ok(executableStat?.isFile(), `Benchmark executable not found: ${executable}`);
await buildResourceSampler(root);

const stamp = new Date().toISOString().replaceAll(':', '-').replaceAll('.', '-');
const outputRoot = path.join(root, 'output', `resource-benchmark-${stamp}`);
await fs.mkdir(outputRoot, { recursive: true });
const intervalMs = 2000;
const readyAccounts = new Set();
const cosmeticAccounts = new Set();
const adPayloadBytes = 64 * 1024;
const fixtureRequests = { ad: 0, content: 0, adResponseBytes: 0, contentResponseBytes: 0 };
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const fixture = http.createServer((request, response) => {
  const url = new URL(request.url, 'http://127.0.0.1');
  response.setHeader('Cache-Control', 'no-store');
  if (url.pathname === '/shell') {
    response.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' });
    response.end('<!doctype html><meta charset="utf-8"><title>Benchmark shell</title>');
    return;
  }
  if (url.pathname === '/fixture') {
    const account = Number(url.searchParams.get('account'));
    const safeAccount = Number.isInteger(account) && account >= 0 && account < 100 ? account : 0;
    response.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' });
    response.end(
      `<!doctype html><meta charset="utf-8"><title>Bobola local benchmark ${safeAccount}</title><style>body{font:14px system-ui;margin:24px;color:#222}main{display:grid;grid-template-columns:repeat(4,1fr);gap:8px}.item{height:42px;background:#e7edf2;border-radius:4px}.sponsored{height:20px;background:#fee}</style><h1>Local service fixture ${safeAccount}</h1><p id="pulse">0</p><div class="sponsored">Sponsored fixture item</div><main>${'<div class="item"></div>'.repeat(48)}</main><script>const account=${JSON.stringify(safeAccount)},started=Date.now();let tick=0;const ready=setInterval(()=>{const hidden=getComputedStyle(document.querySelector('.sponsored')).display==='none';if(hidden||Date.now()-started>5000){clearInterval(ready);fetch('/ready?account='+account+'&cosmetic='+(hidden?'hidden':'visible'));}},100);setInterval(()=>{document.getElementById('pulse').textContent=String(++tick);fetch('/ads/pixel?account='+account+'&tick='+tick).catch(()=>{});fetch('/content/pulse?account='+account+'&tick='+tick).catch(()=>{});},1000);</script>`
    );
    return;
  }
  if (url.pathname === '/ready') {
    const account = Number(url.searchParams.get('account'));
    readyAccounts.add(account);
    if (url.searchParams.get('cosmetic') === 'hidden') cosmeticAccounts.add(account);
    response.writeHead(204).end();
    return;
  }
  if (url.pathname === '/ads/pixel') {
    fixtureRequests.ad++;
    fixtureRequests.adResponseBytes += adPayloadBytes;
    response
      .writeHead(200, {
        'Content-Type': 'application/octet-stream',
        'Content-Length': String(adPayloadBytes),
      })
      .end(Buffer.alloc(adPayloadBytes, 120));
    return;
  }
  if (url.pathname === '/content/pulse') {
    fixtureRequests.content++;
    fixtureRequests.contentResponseBytes += 2;
    response.writeHead(200, { 'Content-Type': 'text/plain', 'Content-Length': '2' }).end('ok');
    return;
  }
  response.writeHead(404).end();
});
await new Promise((resolve) => fixture.listen(0, '127.0.0.1', resolve));
const origin = `http://127.0.0.1:${fixture.address().port}`;

function sampleProcesses(rootPids, profileRoots, durationSeconds, outputPath) {
  const sampler = path.join(root, 'scripts', 'sample-process-tree.ps1');
  const child = spawn(
    'powershell.exe',
    [
      '-NoProfile',
      '-NonInteractive',
      '-ExecutionPolicy',
      'Bypass',
      '-File',
      sampler,
      '-RootPids',
      rootPids.join(','),
      '-ProfileRoots',
      profileRoots.join('|'),
      '-DurationSeconds',
      String(durationSeconds),
      '-IntervalMilliseconds',
      String(intervalMs),
      '-OutputPath',
      outputPath,
    ],
    { stdio: ['ignore', 'ignore', 'pipe'], windowsHide: true }
  );
  let stderr = '';
  child.stderr.on('data', (chunk) => {
    stderr = (stderr + chunk).slice(-4000);
  });
  return new Promise((resolve, reject) => {
    child.once('error', reject);
    child.once('exit', (code) =>
      code === 0 ? resolve() : reject(new Error(`Process sampler failed (${code}): ${stderr}`))
    );
  });
}

function percentile(values, fraction) {
  const sorted = [...values].sort((a, b) => a - b);
  return sorted[Math.min(sorted.length - 1, Math.ceil(sorted.length * fraction) - 1)];
}

async function waitFor(predicate, message, timeoutMs = 90000) {
  const end = Date.now() + timeoutMs;
  while (Date.now() < end) {
    const value = await predicate();
    if (value) return value;
    await sleep(250);
  }
  throw new Error(message);
}

async function directoryBytes(folder) {
  let total = 0;
  const stack = [folder];
  while (stack.length) {
    const current = stack.pop();
    const entries = await fs.readdir(current, { withFileTypes: true }).catch(() => []);
    for (const entry of entries) {
      const full = path.join(current, entry.name);
      if (entry.isDirectory()) stack.push(full);
      else if (entry.isFile()) total += (await fs.stat(full).catch(() => ({ size: 0 }))).size;
    }
  }
  return total;
}

async function benchmarkDataBytes(folder) {
  let total = 0;
  const ignored = new Set([
    'benchmark-filters.txt',
    'benchmark-state.json',
    'measurements.json',
    'process-samples.json',
    'exit-request',
  ]);
  const ignoredDirectories = new Set(['browser', 'shell-browser']);
  const stack = [folder];
  while (stack.length) {
    const current = stack.pop();
    const entries = await fs.readdir(current, { withFileTypes: true }).catch(() => []);
    for (const entry of entries) {
      if (entry.isDirectory() && ignoredDirectories.has(entry.name)) continue;
      if (ignored.has(entry.name)) continue;
      const full = path.join(current, entry.name);
      if (entry.isDirectory()) stack.push(full);
      else if (entry.isFile()) total += (await fs.stat(full).catch(() => ({ size: 0 }))).size;
    }
  }
  return total;
}

async function runCase(name, count, protection, instances = 1, runIndex = 0) {
  assert.equal(count % instances, 0, 'Services must divide evenly across benchmark instances.');
  const runName = `run-${String(runIndex + 1).padStart(2, '0')}`;
  const dir = path.join(outputRoot, name, runName);
  await fs.mkdir(dir, { recursive: true });
  const servicesPerInstance = count / instances;
  const instanceDirs = [];
  const filterFiles = [];
  for (let index = 0; index < instances; index++) {
    const instanceDir = instances === 1 ? dir : path.join(dir, `instance-${index + 1}`);
    await fs.mkdir(instanceDir, { recursive: true });
    instanceDirs.push(instanceDir);
    const filterFile = path.join(instanceDir, 'benchmark-filters.txt');
    await fs.writeFile(
      filterFile,
      '[Adblock Plus 2.0]\n! Local benchmark-only rules\n*/ads/pixel*\n127.0.0.1##.sponsored\n'
    );
    filterFiles.push(filterFile);
  }
  readyAccounts.clear();
  cosmeticAccounts.clear();
  fixtureRequests.ad = 0;
  fixtureRequests.content = 0;
  fixtureRequests.adResponseBytes = 0;
  fixtureRequests.contentResponseBytes = 0;
  const children = instanceDirs.map((instanceDir, index) => {
    const child = spawn(executable, [], {
      windowsHide: false,
      stdio: ['ignore', 'ignore', 'pipe'],
      env: {
        ...process.env,
        SERVICE_WORKSPACE_BENCHMARK_DIR: instanceDir,
        SERVICE_WORKSPACE_BENCHMARK_ORIGIN: origin,
        SERVICE_WORKSPACE_BENCHMARK_COUNT: String(servicesPerInstance),
        SERVICE_WORKSPACE_BENCHMARK_FILTER_FILE: filterFiles[index],
        SERVICE_WORKSPACE_BENCHMARK_PROTECTION: protection ? 'on' : 'off',
        SERVICE_WORKSPACE_BENCHMARK_INDEX: String(index * servicesPerInstance),
      },
    });
    child.stderrText = '';
    child.stderr.on('data', (chunk) => {
      child.stderrText = (child.stderrText + chunk).slice(-4000);
    });
    child.exited = new Promise((resolve) =>
      child.once('exit', (code, signal) => resolve({ code, signal }))
    );
    return child;
  });
  const child = children[0];
  const stderrText = () =>
    children
      .map((entry) => entry.stderrText)
      .filter(Boolean)
      .join('\n');
  try {
    await waitFor(async () => {
      const states = await Promise.all(
        instanceDirs.map((instanceDir) =>
          fs
            .readFile(path.join(instanceDir, 'benchmark-state.json'), 'utf8')
            .then(JSON.parse)
            .catch(() => null)
        )
      );
      return (
        states.length === instances &&
        states.every((state) => state?.loaded === servicesPerInstance)
      );
    }, `${name}: native benchmark views did not initialize. ${stderrText()}`);
    if (count) {
      await waitFor(
        () => readyAccounts.size === count,
        `${name}: fixture pages were not ready. ${stderrText()}`
      );
      if (!protection)
        assert.equal(
          cosmeticAccounts.size,
          0,
          `${name}: cosmetic filters ran while protection was disabled.`
        );
    }
    await sleep(5000);
    const startStates = await Promise.all(
      instanceDirs.map((instanceDir) =>
        fs.readFile(path.join(instanceDir, 'benchmark-state.json'), 'utf8').then(JSON.parse)
      )
    );
    const startRequests = { ...fixtureRequests };
    const samplesPath = path.join(dir, 'process-samples.json');
    const profileRoots = instanceDirs.flatMap((instanceDir) => [
      path.join(instanceDir, 'shell-browser'),
      path.join(instanceDir, 'browser'),
    ]);
    await sampleProcesses(
      children.map((entry) => entry.pid),
      profileRoots,
      sampleSeconds,
      samplesPath
    );
    const parsedSamples = JSON.parse(
      await fs.readFile(samplesPath, 'utf8').then((text) => text.replace(/^\uFEFF/, ''))
    );
    const samples = Array.isArray(parsedSamples) ? parsedSamples : [parsedSamples];
    validateResourceSamples(
      samples,
      children.map((entry) => entry.pid)
    );
    const states = await Promise.all(
      instanceDirs.map((instanceDir) =>
        fs.readFile(path.join(instanceDir, 'benchmark-state.json'), 'utf8').then(JSON.parse)
      )
    );
    const endRequests = { ...fixtureRequests };
    const final = samples.at(-1);
    const { seconds, cpuSeconds: cpuDelta, churnIntervals } = resourceCpu(samples);
    const series = (field) => samples.map((sample) => sample[field]);
    const result = {
      name,
      run: runIndex + 1,
      services: count,
      instances,
      protectionEnabled: protection,
      pageReadyCount: readyAccounts.size,
      cosmeticHiddenCount: cosmeticAccounts.size,
      measurementSeconds: seconds,
      sampleCount: samples.length,
      measurementScope: 'owned-process-tree',
      cpuChurnIntervals: churnIntervals,
      cpuMethod:
        'Matching process lifetimes between consecutive samples; work by appearing or exiting processes is omitted.',
      medianPrivateCommitBytes: percentile(series('privateBytes'), 0.5),
      p95PrivateCommitBytes: percentile(series('privateBytes'), 0.95),
      medianSummedWorkingSetBytes: percentile(series('workingSetBytes'), 0.5),
      p95SummedWorkingSetBytes: percentile(series('workingSetBytes'), 0.95),
      medianProcessCount: percentile(series('processCount'), 0.5),
      cpuCorePercent: seconds ? (cpuDelta / seconds) * 100 : 0,
      cpuMachinePercent: seconds ? (cpuDelta / seconds / os.cpus().length) * 100 : 0,
      blockedRequests:
        states
          .flatMap((state) => Object.values(state.runtime ?? {}))
          .reduce((sum, runtime) => sum + (runtime.blocked ?? 0), 0) -
        startStates
          .flatMap((state) => Object.values(state.runtime ?? {}))
          .reduce((sum, runtime) => sum + (runtime.blocked ?? 0), 0),
      fixtureRequests: Object.fromEntries(
        Object.keys(endRequests).map((key) => [key, endRequests[key] - startRequests[key]])
      ),
      adPayloadBytes,
      executableBytes: executableStat.size,
      profileBytes: 0,
      benchmarkDataBytes: 0,
      processTree: final.processes,
      perInstanceProcessTrees: children.map((entry) => ({
        pid: entry.pid,
        stderr: entry.stderrText,
      })),
    };
    assert.ok(
      count === 0 || (protection ? result.blockedRequests > 0 : result.blockedRequests === 0),
      `${name}: unexpected native blocked-request count ${result.blockedRequests}.`
    );
    assert.ok(
      count === 0 || (protection ? fixtureRequests.ad === 0 : fixtureRequests.ad > 0),
      `${name}: ad-like fixture requests do not match protection state.`
    );
    assert.ok(
      count === 0 || fixtureRequests.content >= count * 5,
      `${name}: first-party fixture requests were not sustained.`
    );
    assert.equal(fixtureRequests.adResponseBytes, fixtureRequests.ad * adPayloadBytes);
    result.benchmarkDataBytes = await benchmarkDataBytes(dir);
    await fs.writeFile(
      path.join(dir, 'measurements.json'),
      JSON.stringify({ result, samples, runtimes: states.map((state) => state.runtime) }, null, 2)
    );
    for (const instanceDir of instanceDirs)
      await fs.writeFile(path.join(instanceDir, 'exit-request'), 'exit');
    const exits = await Promise.race([
      Promise.all(children.map((entry) => entry.exited)),
      sleep(20000).then(() => null),
    ]);
    if (!exits) {
      for (const entry of children) entry.kill();
      throw new Error(`${name}: benchmark apps did not exit after owned exit requests.`);
    }
    for (const exit of exits)
      assert.equal(
        exit.code,
        0,
        `${name}: benchmark app exited with code ${exit.code}. ${stderrText()}`
      );
    result.profileBytes = 0;
    for (const instanceDir of instanceDirs)
      result.profileBytes +=
        (await directoryBytes(path.join(instanceDir, 'shell-browser'))) +
        (await directoryBytes(path.join(instanceDir, 'browser')));
    await fs.writeFile(
      path.join(dir, 'measurements.json'),
      JSON.stringify({ result, samples, runtimes: states.map((state) => state.runtime) }, null, 2)
    );
    return result;
  } catch (error) {
    for (const instanceDir of instanceDirs)
      await fs.writeFile(path.join(instanceDir, 'exit-request'), 'exit').catch(() => {});
    for (const entry of children) {
      if (entry.exitCode === null) {
        await Promise.race([entry.exited, sleep(5000)]);
        if (entry.exitCode === null) entry.kill();
      }
    }
    throw error;
  }
}

try {
  const plan = [
    ['shell-0-services', 0, true],
    ['two-services-protection-on', 2, true],
    ['two-services-protection-off', 2, false],
    ['five-services-protection-on', 5, true],
    ['five-services-protection-off', 5, false],
    ['eight-services-protection-on', 8, true],
    ['five-processes-one-service-each', 5, true, 5],
  ];
  const selectedPlan = selectedCases ? plan.filter(([name]) => selectedCases.includes(name)) : plan;
  assert.ok(selectedPlan.length > 0, 'At least one known benchmark case must be selected.');
  if (selectedCases)
    assert.equal(
      selectedPlan.length,
      selectedCases.length,
      'Unknown or duplicate benchmark case selected.'
    );
  const cases = [];
  for (let runIndex = 0; runIndex < runCount; runIndex++) {
    const ordered = runIndex % 2 === 0 ? selectedPlan : [...selectedPlan].reverse();
    for (const [name, count, protection, instances = 1] of ordered) {
      console.log(
        `Measuring ${name} (${instances} instance${instances === 1 ? '' : 's'}), run ${runIndex + 1}/${runCount}, for ${sampleSeconds}s...`
      );
      cases.push(await runCase(name, count, protection, instances, runIndex));
    }
  }
  const report = {
    status: 'passed',
    generatedAt: new Date().toISOString(),
    executable,
    executableBytes: executableStat.size,
    system: JSON.parse(
      execFileSync(
        'powershell.exe',
        [
          '-NoProfile',
          '-NonInteractive',
          '-Command',
          '@{os=(Get-CimInstance Win32_OperatingSystem).Caption;physicalMemoryBytes=(Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory;cpu=(Get-CimInstance Win32_Processor | Select-Object -First 1 -ExpandProperty Name);logicalProcessors=(Get-CimInstance Win32_ComputerSystem).NumberOfLogicalProcessors;architecture=$env:PROCESSOR_ARCHITECTURE} | ConvertTo-Json -Compress',
        ],
        { encoding: 'utf8', windowsHide: true }
      ).trim()
    ),
    fixture:
      'Owned loopback pages with 48 static DOM tiles, one ad-like 64 KiB response and one 2-byte first-party response per second per service.',
    sampleSeconds,
    runCount,
    sampleIntervalMs: intervalMs,
    metricNotes: {
      privateCommit: 'Sum of private committed bytes across the Bobola root process tree.',
      workingSet: 'Sum of process working sets; shared pages can be double-counted.',
      cpu: 'Process-tree CPU seconds accumulated during the measured window, expressed as one-core and whole-machine percentages.',
      profileBytes:
        'Bytes under the isolated shell-browser and service browser directories after shutdown. It contains no user sign-in data.',
      benchmarkDataBytes:
        'Fresh SQLite workspace and benchmark fixture files only, excluding result files, profile files, and process samples.',
      comparability:
        'Synthetic local pages measure Bobola resource scaling, not RAM saved against another client.',
    },
    cases,
    caseSummaries: selectedPlan.map(([name, services, protection, instances = 1]) => {
      const matching = cases.filter((entry) => entry.name === name);
      const medianOf = (field) =>
        percentile(
          matching.map((entry) => entry[field]),
          0.5
        );
      const medianNested = (field, key) =>
        percentile(
          matching.map((entry) => entry[field][key]),
          0.5
        );
      return {
        name,
        services,
        protectionEnabled: protection,
        instances,
        runCount: matching.length,
        medianOfRunMedianPrivateCommitBytes: medianOf('medianPrivateCommitBytes'),
        medianOfRunMedianWorkingSetBytes: medianOf('medianSummedWorkingSetBytes'),
        medianOfRunMedianProfileBytes: medianOf('profileBytes'),
        medianOfRunMedianProcessCount: medianOf('medianProcessCount'),
        medianOfRunMedianCpuCorePercent: medianOf('cpuCorePercent'),
        medianOfRunMedianCpuMachinePercent: medianOf('cpuMachinePercent'),
        medianOfRunBlockedRequests: medianOf('blockedRequests'),
        medianOfRunAdFixtureRequests: medianNested('fixtureRequests', 'ad'),
        medianOfRunAdFixtureResponseBytes: medianNested('fixtureRequests', 'adResponseBytes'),
        medianOfRunFirstPartyFixtureRequests: medianNested('fixtureRequests', 'content'),
      };
    }),
  };
  const reportPath = path.join(outputRoot, 'resource-results.json');
  await fs.writeFile(reportPath, JSON.stringify(report, null, 2));
  console.log(
    JSON.stringify(
      {
        status: report.status,
        output: reportPath,
        cases: report.caseSummaries,
      },
      null,
      2
    )
  );
} finally {
  await new Promise((resolve) => fixture.close(resolve));
}
