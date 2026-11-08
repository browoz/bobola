import fs from 'node:fs/promises';
import path from 'node:path';
const source = process.argv[2];
if (!source) throw new Error('Pass the path to the pinned ferdium-recipes/recipes folder.');
const entries = [];
for (const folder of await fs.readdir(source, { withFileTypes: true })) {
  if (!folder.isDirectory()) continue;
  try {
    const metadata = JSON.parse(
      await fs.readFile(path.join(source, folder.name, 'package.json'), 'utf8')
    );
    if (!metadata.name || !metadata.id) continue;
    const url = metadata.config?.serviceURL ?? '';
    entries.push({
      id: metadata.id,
      name: metadata.name,
      url,
      needsCustomUrl: !/^https?:\/\//.test(url) || /[{}]/.test(url),
      license: metadata.license ?? 'UNSPECIFIED',
    });
  } catch (error) {
    throw new Error(`Could not read recipe metadata ${folder.name}: ${error.message}`);
  }
}
entries.sort((a, b) => a.name.localeCompare(b.name));
await fs.writeFile(
  new URL('../src/lib/catalog.json', import.meta.url),
  JSON.stringify(entries, null, 2) + '\n'
);
await fs.writeFile(
  new URL('../CATALOG_PROVENANCE.json', import.meta.url),
  JSON.stringify(
    {
      source: 'https://github.com/ferdium/ferdium-recipes',
      revision: 'bc708f6aad02fc43168e976d1d65678eaf96f21b',
      count: entries.length,
      content: 'URL metadata only; upstream recipe scripts and artwork are not bundled.',
      licenses: [...new Set(entries.map((entry) => entry.license))],
    },
    null,
    2
  ) + '\n'
);
console.log(
  `Imported ${entries.length} catalog entries. Site-specific recipe behavior remains tracked separately.`
);
