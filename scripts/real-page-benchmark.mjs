import { spawn } from 'node:child_process';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  buildResourceSampler,
  validateResourceSamples,
  resourceCpu,
} from './resource-measurements.mjs';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const executable = path.resolve(
  process.argv[2] ??
    path.join(
      process.env.CARGO_TARGET_DIR ?? path.join(os.tmpdir(), 'service-workspace-target'),
      'release',
      'service-workspace.exe'
    )
);
const sampleSeconds = Number(process.argv[3] ?? 30);
const runCount = Number(process.argv[4] ?? 3);
const ferdiumSeed = path.resolve(process.argv[5] ?? path.join(os.tmpdir(), 'ferdium-real-pages-seven'));
const protectionMode = process.argv[6] ?? 'on';
if (!Number.isInteger(sampleSeconds) || sampleSeconds < 10 || sampleSeconds > 120)
  throw new Error('Sample duration must be an integer from 10-120 seconds.');
if (!Number.isInteger(runCount) || runCount < 1 || runCount > 10)
  throw new Error('Run count must be an integer from 1-10.');
if (!['on', 'off'].includes(protectionMode)) throw new Error('Protection mode must be on or off.');
const outputRoot = path.resolve(
  process.env.REAL_PAGE_BENCHMARK_OUTPUT ??
    path.join(
      'E:/Temp',
      `service-workspace-real-page-benchmark-${new Date().toISOString().replaceAll(':', '-').replaceAll('.', '-')}`
    )
);
const ferdiumProbe = path.join(root, 'scripts', 'probe-ferdium-profile.mjs');
const sampler = path.join(root, 'scripts', 'sample-process-tree.ps1');
const urls = [
  ['WhatsApp', 'https://web.whatsapp.com/'],
  ['Telegram', 'https://web.telegram.org/a/'],
  ['YouTube', 'https://www.youtube.com/'],
  ['Reddit', 'https://www.reddit.com/'],
  ['Google Gemini', 'https://gemini.google.com/app'],
  ['Discord', 'https://discord.com/app'],
  ['YouTube Music', 'https://music.youtube.com/'],
];
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const readJson = async (file) =>
  JSON.parse(await fs.readFile(file, 'utf8').then((text) => text.replace(/^\uFEFF/, '')));
const filterFile = path.join(outputRoot, 'easylist-easyprivacy.txt');
const pageUrls = JSON.stringify(urls);

await fs.mkdir(outputRoot, { recursive: true });
await buildResourceSampler(root);
const filters = [];
for (const url of [
  'https://easylist.to/easylist/easylist.txt',
  'https://easylist.to/easylist/easyprivacy.txt',
]) {
  const response = await fetch(url);
  if (!response.ok) throw new Error(`Could not download ${url}: ${response.status}`);
  filters.push(await response.text());
}
await fs.writeFile(filterFile, `${filters.join('\n')}\n`, 'utf8');

function sampleProcesses(rootPid, profileRoot, outputPath) {
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
      String(rootPid),
      '-ProfileRoots',
      profileRoot,
      '-DurationSeconds',
      String(sampleSeconds),
      '-IntervalMilliseconds',
      '2000',
      '-OutputPath',
      outputPath,
    ],
    { windowsHide: true, stdio: ['ignore', 'ignore', 'pipe'] }
  );
  let stderr = '';
  child.stderr.on('data', (chunk) => {
    stderr = `${stderr}${chunk}`.slice(-4000);
  });
  return new Promise((resolve, reject) => {
    child.once('error', reject);
    child.once('exit', (code) =>
      code === 0 ? resolve() : reject(new Error(`sampler exited ${code}: ${stderr}`))
    );
  });
}

function killProcessTree(pid) {
  return new Promise((resolve) => {
    const killer = spawn('taskkill.exe', ['/PID', String(pid), '/T', '/F'], {
      windowsHide: true,
      stdio: 'ignore',
    });
    killer.once('error', resolve);
    killer.once('exit', resolve);
  });
}

async function waitFor(file, predicate, label, timeout = 240000) {
  const deadline = Date.now() + timeout;
  while (Date.now() < deadline) {
    const value = await readJson(file).catch(() => null);
    if (value && predicate(value)) return value;
    await sleep(500);
  }
  throw new Error(`Timed out waiting for ${label}`);
}

async function stopProcess(child, marker) {
  await fs.writeFile(marker, 'exit', 'utf8').catch(() => {});
  await Promise.race([new Promise((resolve) => child.once('exit', resolve)), sleep(5000)]);
  if (child.exitCode === null) await killProcessTree(child.pid);
}

async function runBobola(index, protection) {
  const dir = path.join(outputRoot, 'bobola', `run-${index}`);
  await fs.mkdir(dir, { recursive: true });
  const filter = protection ? filterFile : path.join(dir, 'empty-filters.txt');
  if (!protection) await fs.writeFile(filter, '[Adblock Plus 2.0]\n||benchmark.invalid^\n', 'utf8');
  const child = spawn(executable, [], {
    windowsHide: true,
    stdio: ['ignore', 'ignore', 'pipe'],
    env: {
      ...process.env,
      SERVICE_WORKSPACE_BENCHMARK_DIR: dir,
      SERVICE_WORKSPACE_BENCHMARK_URLS: pageUrls,
      SERVICE_WORKSPACE_BENCHMARK_FILTER_FILE: filter,
      SERVICE_WORKSPACE_BENCHMARK_PROTECTION: protection ? protectionMode : 'off',
    },
  });
  let stderr = '';
  child.stderr.on('data', (chunk) => {
    stderr = `${stderr}${chunk}`.slice(-6000);
  });
  try {
    const stateFile = path.join(dir, 'benchmark-state.json');
    const ready = await waitFor(
      stateFile,
      (state) =>
        state.loaded === urls.length &&
        Object.keys(state.navigationCompletedMs ?? {}).length === urls.length,
      'Bobola pages'
    );
    await sleep(15000);
    const start = await readJson(stateFile);
    const samplesPath = path.join(dir, 'process-samples.json');
    await sampleProcesses(child.pid, dir, samplesPath);
    const end = await readJson(stateFile);
    const samples = await readJson(samplesPath);
    const normalizedSamples = Array.isArray(samples) ? samples : [samples];
    validateResourceSamples(normalizedSamples, [child.pid]);
    const navigation = end.navigationCompletedMs ?? {};
    const result = {
      app: 'Bobola',
      run: index,
      protection,
      ready: {
        serviceCount: end.configured,
        loaded: end.loaded,
        readyAfterMs: Math.max(0, ...Object.values(navigation)),
        navigationCompletedMs: navigation,
        traffic: end.traffic ?? {},
      },
      start,
      end,
      samples: normalizedSamples,
      cpu: resourceCpu(normalizedSamples),
      stderr,
    };
    await stopProcess(child, path.join(dir, 'exit-request'));
    await fs.writeFile(path.join(dir, 'result.json'), JSON.stringify(result, null, 2), 'utf8');
    return result;
  } catch (error) {
    await stopProcess(child, path.join(dir, 'exit-request'));
    throw new Error(`Bobola run ${index} failed: ${error.message}\n${stderr}`);
  }
}

async function runFerdium(index) {
  const dir = path.join(outputRoot, 'ferdium', `run-${index}`);
  const profile = path.join(dir, 'profile');
  await fs.mkdir(dir, { recursive: true });
  // The seed is already a verified seven-service profile. Copying it gives each
  // measurement its own app data while retaining the same service definitions.
  await fs.cp(ferdiumSeed, profile, { recursive: true });
  // Keep the verified service definitions and app settings, but give each run
  // fresh Chromium partitions. Copying live IndexedDB/CacheStorage trees makes
  // Chromium reject stale cache indexes and changes the cold-profile condition.
  await fs.rm(path.join(profile, 'Partitions'), { recursive: true, force: true });
  // Chromium profile databases copied from a live seed can inherit read-only
  // attributes on Windows. Clear them before Electron opens the profile.
  await new Promise((resolve) => {
    const attributes = spawn('attrib', ['-R', `${profile}\\*`, '/S', '/D'], { windowsHide: true });
    attributes.once('error', resolve);
    attributes.once('exit', resolve);
  });
  const readyFile = path.join(dir, 'ready.json');
  const child = spawn(
    process.execPath,
    [
      ferdiumProbe,
      `--profile=${profile}`,
      '--wait-pages',
      `--hold=${sampleSeconds + 20}`,
      `--ready-file=${readyFile}`,
    ],
    {
      windowsHide: true,
      stdio: ['ignore', 'pipe', 'pipe'],
    }
  );
  let stdout = '',
    stderr = '';
  child.stdout.on('data', (chunk) => {
    stdout = `${stdout}${chunk}`.slice(-12000);
  });
  child.stderr.on('data', (chunk) => {
    stderr = `${stderr}${chunk}`.slice(-6000);
  });
  try {
    const ready = await waitFor(
      readyFile,
      (value) =>
        value.serviceCount === urls.length &&
        value.services?.every((service) => service.hasWebview && !service.error),
      'Ferdium pages'
    );
    await sleep(15000);
    const samplesPath = path.join(dir, 'process-samples.json');
    await sampleProcesses(ready.pid, profile, samplesPath);
    const samples = await readJson(samplesPath);
    const normalizedSamples = Array.isArray(samples) ? samples : [samples];
    validateResourceSamples(normalizedSamples, [ready.pid]);
    const result = {
      app: 'Ferdium',
      run: index,
      protection: false,
      ready,
      samples: normalizedSamples,
      cpu: resourceCpu(normalizedSamples),
      stdout,
      stderr,
    };
    if (child.exitCode === null) await killProcessTree(child.pid);
    await fs.writeFile(path.join(dir, 'result.json'), JSON.stringify(result, null, 2), 'utf8');
    return result;
  } catch (error) {
    if (child.exitCode === null) await killProcessTree(child.pid);
    throw new Error(`Ferdium run ${index} failed: ${error.message}\n${stderr}`);
  }
}

const results = [];
try {
  for (let index = 1; index <= runCount; index += 1) {
    // Alternate order so launch/cache and background-load drift is distributed.
    if (index % 2) {
      console.log(`Run ${index}/${runCount}: Bobola`);
      results.push(await runBobola(index, protectionMode === 'on'));
      console.log(`Run ${index}/${runCount}: Ferdium`);
      results.push(await runFerdium(index));
    } else {
      console.log(`Run ${index}/${runCount}: Ferdium`);
      results.push(await runFerdium(index));
      console.log(`Run ${index}/${runCount}: Bobola`);
      results.push(await runBobola(index, protectionMode === 'on'));
    }
  }
  await fs.writeFile(
    path.join(outputRoot, 'real-page-results.json'),
    JSON.stringify(
      {
        generatedAt: new Date().toISOString(),
        executable,
        urls,
        filterFile,
        filterSources: [
          'https://easylist.to/easylist/easylist.txt',
          'https://easylist.to/easylist/easyprivacy.txt',
        ],
        sampleSeconds,
        runCount,
        measurementScope: 'owned-process-tree',
        cpuMethod:
          'Matching process lifetimes between consecutive samples; work by appearing or exiting processes is omitted.',
        results,
      },
      null,
      2
    ),
    'utf8'
  );
  console.log(JSON.stringify({ outputRoot, resultCount: results.length }, null, 2));
} finally {
  // No external process is owned by this script after each run's cleanup path.
}
