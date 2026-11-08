import { execFileSync } from 'node:child_process';
import { readFile, readdir, writeFile } from 'node:fs/promises';
import path from 'node:path';

// Resolve only the Windows dependency graph actually used by this application.
const metadata = JSON.parse(
  execFileSync(
    'cargo',
    ['metadata', '--format-version', '1', '--filter-platform', 'x86_64-pc-windows-msvc'],
    { encoding: 'utf8', maxBuffer: 20 * 1024 * 1024 }
  )
);
const ids = new Set(metadata.resolve.nodes.map((node) => node.id));
const sections = [
  'Bobola dependency notices\nGenerated from Cargo.lock and package-lock.json.\nDependency code retains the licenses below.\n',
];
let count = 0;
async function licenses(folder) {
  const entries = await readdir(folder, { withFileTypes: true });
  const files = entries.filter(
    (entry) => entry.isFile() && /^(licen[cs]e|copying|notice|copyright)([._-]|$)/i.test(entry.name)
  );
  return Promise.all(
    files.map(
      async (entry) => `${entry.name}\n${await readFile(path.join(folder, entry.name), 'utf8')}`
    )
  );
}
for (const pkg of metadata.packages
  .filter((pkg) => ids.has(pkg.id) && pkg.source)
  .sort((a, b) => a.name.localeCompare(b.name))) {
  const texts = await licenses(path.dirname(pkg.manifest_path));
  sections.push(
    `\n${'='.repeat(72)}\n${pkg.name} ${pkg.version}\nLicense: ${pkg.license ?? 'See source'}\nSource: ${pkg.repository ?? pkg.source}\nArchive: https://crates.io/api/v1/crates/${pkg.name}/${pkg.version}/download\n${texts.join('\n\n') || 'License text is available in the source archive above.'}`
  );
  count++;
}
const lock = JSON.parse(await readFile('package-lock.json', 'utf8'));
for (const [folder, pkg] of Object.entries(lock.packages).filter(
  ([folder, pkg]) => folder && !pkg.dev && !pkg.optional
)) {
  const manifest = JSON.parse(await readFile(path.join(folder, 'package.json'), 'utf8'));
  sections.push(
    `\n${'='.repeat(72)}\n${manifest.name} ${pkg.version}\nLicense: ${manifest.license ?? pkg.license}\nSource: ${pkg.resolved}\n${(await licenses(folder)).join('\n\n')}`
  );
  count++;
}
await writeFile('docs/DEPENDENCY_NOTICES.txt', sections.join('\n'));
console.log(`Generated notices for ${count} Rust and production npm packages.`);
