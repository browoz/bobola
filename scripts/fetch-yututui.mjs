import { createHash } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const version = '1.7.9';
const root = fileURLToPath(new URL('..', import.meta.url));
const resourceDir = path.join(root, 'src-tauri', 'resources');
const archive = path.join(resourceDir, `yututui-windows-x64-${version}.zip`);
const binary = path.join(resourceDir, 'ytt.exe');
const license = path.join(resourceDir, 'YUTUTUI_LICENSE.md');
const url = `https://github.com/Ochichan/Yututui/releases/download/v${version}/yututui-windows-x64.zip`;

if ((await readFile(binary).catch(() => null))?.length) {
  console.log(`YuTuTui ${version} is already available.`);
  process.exit(0);
}

await mkdir(resourceDir, { recursive: true });
const response = await fetch(url);
if (!response.ok) throw new Error(`YuTuTui download failed: ${response.status} ${response.statusText}`);
const bytes = Buffer.from(await response.arrayBuffer());
const expected = '2fde16fb70e9b31f7227aee5b464ba1202808d2c67e186bb8b434cdd8b73b5d7';
const actual = createHash('sha256').update(bytes).digest('hex');
if (actual !== expected) {
  throw new Error(`YuTuTui checksum mismatch for v${version}: expected ${expected}, got ${actual}`);
}
await writeFile(archive, bytes);
const extract = `Expand-Archive -LiteralPath '${archive.replaceAll("'", "''")}' -DestinationPath '${resourceDir.replaceAll("'", "''")}' -Force`;
execFileSync('powershell.exe', ['-NoProfile', '-Command', extract], { stdio: 'inherit' });
if (!(await readFile(binary).catch(() => null))?.length) throw new Error('YuTuTui archive did not contain ytt.exe.');
await writeFile(
  license,
  `MIT License\n\nCopyright (c) 2026 Ochichan\n\nPermission is hereby granted, free of charge, to any person obtaining a copy\nof this software and associated documentation files (the "Software"), to deal\nin the Software without restriction, including without limitation the rights\nto use, copy, modify, merge, publish, distribute, sublicense, and/or sell\ncopies of the Software, and to permit persons to whom the Software is\nfurnished to do so, subject to the following conditions:\n\nThe above copyright notice and this permission notice shall be included in all\ncopies or substantial portions of the Software.\n\nTHE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR\nIMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,\nFITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE\nAUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER\nLIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,\nOUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE\nSOFTWARE.\n`
);
console.log(`Downloaded YuTuTui ${version} (${actual}).`);
