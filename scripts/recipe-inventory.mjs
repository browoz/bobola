import fs from 'node:fs/promises';
import path from 'node:path';
const root = process.argv[2];
if (!root) throw new Error('Pass the pinned recipes directory.');
const recipes = [];
for (const folder of await fs.readdir(root, { withFileTypes: true })) {
  if (!folder.isDirectory()) continue;
  const directory = path.join(root, folder.name);
  const metadata = JSON.parse(await fs.readFile(path.join(directory, 'package.json'), 'utf8'));
  const scripts = [];
  for (const name of ['index.js', 'webview.js']) {
    try {
      scripts.push(await fs.readFile(path.join(directory, name), 'utf8'));
    } catch (error) {
      if (error.code !== 'ENOENT') throw error;
    }
  }
  const text = scripts.join('\n');
  const apiCalls = [
    ...new Set(
      [...text.matchAll(/\b(?:Franz|Ferdi|Ferdium)\.([a-zA-Z_$][\w$]*)\s*\(/g)].map(
        (match) => match[1]
      )
    ),
  ].sort();
  const imports = [
    ...new Set([...text.matchAll(/require\(['"]([^'"]+)['"]\)/g)].map((match) => match[1])),
  ].sort();
  recipes.push({
    id: metadata.id,
    name: metadata.name,
    apiCalls,
    imports,
    requiresBrowserAdapter: !!text.trim(),
    adapterStatus: 'not-ported',
    catalogMetadata: 'imported',
    runtimeVerified: false,
  });
}
const summary = {
  revision: 'bc708f6aad02fc43168e976d1d65678eaf96f21b',
  recipeCount: recipes.length,
  withImports: recipes.filter((recipe) => recipe.imports.length).length,
  methods: [...new Set(recipes.flatMap((recipe) => recipe.apiCalls))].sort(),
  limitation: 'Literal pattern inventory only; computed imports and aliases require manual review.',
  recipes,
};
await fs.writeFile(
  new URL('../RECIPE_SUPPORT.json', import.meta.url),
  JSON.stringify(summary, null, 2) + '\n'
);
console.log(
  JSON.stringify({
    count: summary.recipeCount,
    withImports: summary.withImports,
    methods: summary.methods,
  })
);
