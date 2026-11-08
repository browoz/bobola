import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import path from 'node:path';

export async function buildResourceSampler(root) {
  await new Promise((resolve, reject) => {
    const child = spawn(
      process.execPath,
      [
        path.join(root, 'scripts', 'rust.mjs'),
        'build',
        '-p',
        'service-workspace',
        '--bin',
        'resource-sampler',
      ],
      { cwd: root, stdio: 'inherit', windowsHide: true }
    );
    child.once('error', reject);
    child.once('exit', (code) =>
      code === 0 ? resolve() : reject(new Error(`Native sampler build failed (${code}).`))
    );
  });
}

export function validateResourceSamples(samples, rootPids) {
  assert.ok(samples.length >= 2, 'At least two resource samples are required.');
  for (const sample of samples) {
    assert.equal(
      sample.scope,
      'owned-process-tree',
      'Resource sample has the wrong measurement scope.'
    );
    assert.equal(
      sample.unavailable,
      0,
      'Incomplete measurements cannot be used for a benchmark total.'
    );
    assert.equal(
      new Set(sample.processes.map((process) => process.pid)).size,
      sample.processes.length,
      'Duplicate process IDs in resource total.'
    );
    for (const pid of rootPids)
      assert.ok(
        sample.processes.some((process) => process.pid === pid),
        `Owned root PID ${pid} disappeared.`
      );
    assert.ok(
      sample.processes.some(
        (process) =>
          /^(msedgewebview2|ferdium)\.exe$/i.test(process.name) && !rootPids.includes(process.pid)
      ),
      'Browser children are missing from resource measurements.'
    );
    assert.ok(
      sample.processes.every(
        (process) =>
          process.privateBytes !== null && process.cpuSeconds !== null && process.createdAt100ns
      ),
      'Process measurements are incomplete.'
    );
  }
}

// Process lifetime identity avoids counting a reused PID or subtracting changing
// process-tree totals. Processes absent at either endpoint are explicitly omitted.
export function resourceCpu(samples) {
  let cpuSeconds = 0,
    seconds = 0,
    churnIntervals = 0;
  const identity = (process) => `${process.pid}:${process.createdAt100ns}`;
  for (let index = 1; index < samples.length; index++) {
    const before = samples[index - 1],
      after = samples[index];
    const elapsed = (Date.parse(after.utc) - Date.parse(before.utc)) / 1000;
    if (elapsed <= 0) continue;
    seconds += elapsed;
    const previous = new Map(before.processes.map((process) => [identity(process), process]));
    const current = new Set(after.processes.map(identity));
    if (previous.size !== current.size || [...previous.keys()].some((key) => !current.has(key)))
      churnIntervals++;
    for (const process of after.processes) {
      const earlier = previous.get(identity(process));
      if (
        process.cpuSeconds != null &&
        earlier?.cpuSeconds != null &&
        process.cpuSeconds >= earlier.cpuSeconds
      )
        cpuSeconds += process.cpuSeconds - earlier.cpuSeconds;
    }
  }
  return { cpuSeconds, seconds, churnIntervals };
}
