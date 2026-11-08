import { createHash } from 'node:crypto';
import { readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';

// Pass the installer, distribution ZIP, and source ZIP to produce one checksum file.
const files = process.argv.slice(2);
if (!files.length) throw new Error('Provide release artifact paths.');
const lines = [];
for (const file of files) {
  const bytes = await readFile(file);
  lines.push(`${createHash('sha256').update(bytes).digest('hex')}  ${path.basename(file)}`);
}
await writeFile('output/SHA256SUMS.txt', `${lines.join('\n')}\n`);
console.log(lines.join('\n'));
