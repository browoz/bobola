import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { copyFile, lstat, mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import path from 'node:path';

const { version } = JSON.parse(await readFile('package.json', 'utf8'));
const destination = path.resolve('output', `bobola-${version}-source`);
const archive = `${destination}.zip`;
const rootFiles = new Set([
  '.gitattributes',
  '.gitignore',
  '.prettierignore',
  '.prettierrc.json',
  'Cargo.lock',
  'Cargo.toml',
  'CATALOG_PROVENANCE.json',
  'deny.toml',
  'index.html',
  'LICENSE',
  'package-lock.json',
  'package.json',
  'README.md',
  'CONTRIBUTING.md',
  'RECIPE_SUPPORT.json',
  'SERVICE_ARTWORK_PROVENANCE.json',
  'svelte.config.js',
  'THIRD_PARTY_NOTICES.md',
  'tsconfig.json',
  'vite.config.ts',
]);
const docs = new Set([
  'DEPENDENCY_NOTICES.txt',
  'RELEASE_NOTES_1.1.0.md',
  'UI_REVIEW_1.1.0.md',
  `RELEASE_NOTES_${version}.md`,
  `UI_REVIEW_${version}.md`,
  'BENCHMARK_ANALYSIS.md',
  'PARITY.md',
  'IMPLEMENTATION.md',
]);
const candidates = execFileSync('git', ['ls-files', '-c', '-o', '--exclude-standard', '-z'], {
  encoding: 'utf8',
})
  .split('\0')
  .filter(Boolean);
const files = [...new Set(candidates)]
  .filter(
    (file) =>
      rootFiles.has(file) ||
      /^\.github\//.test(file) ||
      /^docs\/assets\//.test(file) ||
      /^(src|crates)\//.test(file) ||
      /^src-tauri\/(src\/|icons\/|capabilities\/|Cargo\.toml$|build\.rs$|tauri\.conf\.json$|nsis-hooks\.nsh$)/.test(
        file
      ) ||
      (/^scripts\/[a-z0-9-]+\.(mjs|ps1)$/.test(file) &&
        !/(deliver-bobola|sonnet|probe-ferdium|render-confluence|omp-)/.test(file)) ||
      /^public\/(app-icon\.svg$|bobola-logo\.svg$|service-icons\/)/.test(file) ||
      (file.startsWith('docs/') && docs.has(file.slice(5)))
  )
  .sort();
const secrets = [
  /-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----/,
  /\b(?:gh[pousr]_[A-Za-z0-9]{30,}|github_pat_[A-Za-z0-9_]{40,})\b/,
  /\bAKIA[A-Z0-9]{16}\b/,
  /\bxox[baprs]-[A-Za-z0-9-]{20,}\b/,
];
const manifest = [];
// Validate every candidate before creating the output, so failures leave no partial package.
const checked = [];
for (const file of files) {
  const info = await lstat(file).catch(() => null);
  if (!info) continue; // Deleted tracked files are not public candidates.
  assert.ok(info.isFile() && !info.isSymbolicLink(), `Unexpected source entry: ${file}`);
  const bytes = await readFile(file);
  if (!/\.(png|ico|icns)$/.test(file)) {
    const text = bytes.toString('utf8');
    assert.ok(!secrets.some((pattern) => pattern.test(text)), `Possible credential in ${file}`);
    assert.ok(!/[A-Z]:[\\/](?:Users|Temp)[\\/]/i.test(text), `Personal absolute path in ${file}`);
  }
  checked.push(file);
  manifest.push({
    path: file,
    bytes: bytes.length,
    sha256: createHash('sha256').update(bytes).digest('hex'),
  });
}
assert.ok(checked.includes(`docs/UI_REVIEW_${version}.md`), 'The release review must be included.');
// An existing output could contain stale files; require a fresh destination.
await rm(destination, { recursive: true, force: true });
await rm(archive, { force: true });
await mkdir(path.dirname(destination), { recursive: true });
await mkdir(destination);
for (const file of checked) {
  const target = path.join(destination, file);
  await mkdir(path.dirname(target), { recursive: true });
  await copyFile(file, target);
}
await writeFile(
  path.join(destination, 'SOURCE_MANIFEST.json'),
  JSON.stringify({ version, generatedAt: new Date().toISOString(), files: manifest }, null, 2)
);
const quote = (value) => `'${value.replaceAll("'", "''")}'`;
const ps = `Add-Type -AssemblyName System.IO.Compression.FileSystem; [IO.Compression.ZipFile]::CreateFromDirectory(${quote(destination)}, ${quote(archive)})`;
execFileSync('powershell.exe', [
  '-NoProfile',
  '-EncodedCommand',
  Buffer.from(ps, 'utf16le').toString('base64'),
]);
const sha256 = createHash('sha256')
  .update(await readFile(archive))
  .digest('hex');
await writeFile(`${archive}.sha256`, `${sha256}  ${path.basename(archive)}\n`);
console.log(
  JSON.stringify(
    {
      archive,
      files: manifest.length,
      sha256,
      secretPatternScan: 'passed',
      personalPathScan: 'passed',
    },
    null,
    2
  )
);
