import fs from 'node:fs/promises';
import path from 'node:path';
import os from 'node:os';
import crypto from 'node:crypto';
import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const { version } = JSON.parse(await fs.readFile(path.join(root, 'package.json'), 'utf8'));
const installerName = `Bobola_${version}_x64-setup.exe`;
const targetCandidates = [
  process.env.CARGO_TARGET_DIR,
  path.join(root, 'target'),
  path.join(os.tmpdir(), 'service-workspace-target'),
].filter(Boolean);
let installer;
for (const candidate of targetCandidates) {
  const possible = path.join(candidate, 'release', 'bundle', 'nsis', installerName);
  if ((await fs.stat(possible).catch(() => null))?.isFile()) {
    installer = possible;
    break;
  }
}
if (!installer) throw new Error(`Windows installer not found in: ${targetCandidates.join(', ')}`);
const destination = path.join(root, 'output', `bobola-${version}-windows-x64`);
const archive = path.join(root, 'output', `bobola-${version}-windows-x64.zip`);
const smoke = JSON.parse(
  await fs.readFile(path.join(root, 'output/native/smoke-result.json'), 'utf8')
);
if (smoke.status !== 'passed') throw new Error('A passing native smoke report is required.');

await fs.rm(destination, { recursive: true, force: true });
await fs.mkdir(destination, { recursive: true });
const names = [
  installerName,
  'LICENSE',
  'THIRD_PARTY_NOTICES.md',
  `RELEASE_NOTES_${version}.md`,
  'DEPENDENCY_NOTICES.txt',
  'BUILD_MANIFEST.json',
  'SHA256SUMS.txt',
];

await fs.copyFile(installer, path.join(destination, names[0]));
await fs.copyFile(path.join(root, 'LICENSE'), path.join(destination, 'LICENSE'));
await fs.copyFile(
  path.join(root, 'THIRD_PARTY_NOTICES.md'),
  path.join(destination, 'THIRD_PARTY_NOTICES.md')
);
await fs.copyFile(
  path.join(root, 'docs', `RELEASE_NOTES_${version}.md`),
  path.join(destination, `RELEASE_NOTES_${version}.md`)
);
await fs.copyFile(
  path.join(root, 'docs', 'DEPENDENCY_NOTICES.txt'),
  path.join(destination, 'DEPENDENCY_NOTICES.txt')
);

const bytes = await fs.readFile(path.join(destination, names[0]));
const sha256 = crypto.createHash('sha256').update(bytes).digest('hex');
const manifest = {
  product: 'Bobola',
  version,
  platform: 'windows-x64',
  artifact: names[0],
  bytes: bytes.length,
  sha256,
  distribution: 'per-user NSIS installer',
  webview2: 'download bootstrapper',
  signed: false,
  validation: {
    nativeSmoke: { status: smoke.status, checks: smoke.checks },
    review:
      `See docs/UI_REVIEW_${version}.md in the source package for validation evidence and limitations.`,
  },
};
await fs.writeFile(
  path.join(destination, 'BUILD_MANIFEST.json'),
  JSON.stringify(manifest, null, 2) + '\n'
);
await fs.writeFile(path.join(destination, 'SHA256SUMS.txt'), `${sha256}  ${names[0]}\n`);

const quote = (value) => `'${value.replaceAll("'", "''")}'`;
const script = `Compress-Archive -LiteralPath ${names
  .map((name) => quote(path.join(destination, name)))
  .join(',')} -DestinationPath ${quote(archive)} -Force`;
await new Promise((resolve, reject) => {
  const child = spawn(
    'powershell.exe',
    ['-NoProfile', '-EncodedCommand', Buffer.from(script, 'utf16le').toString('base64')],
    { windowsHide: true, stdio: 'inherit' }
  );
  child.on('error', reject);
  child.on('exit', (code) =>
    code === 0 ? resolve() : reject(new Error(`Archive creation exited with ${code}`))
  );
});

console.log(
  JSON.stringify({
    installer: path.join(destination, names[0]),
    archive,
    bytes: bytes.length,
    sha256,
  })
);
