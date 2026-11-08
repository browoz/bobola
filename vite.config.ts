import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { readFile, readdir } from 'node:fs/promises';
export default defineConfig({
  // Emit only application assets; local design experiments may also live in public/.
  build: { target: 'es2022', copyPublicDir: false },
  plugins: [
    svelte(),
    {
      name: 'release-assets',
      apply: 'build',
      async generateBundle() {
        const icons = (await readdir('public/service-icons')).filter((name) =>
          /\.(svg|txt)$/.test(name)
        );
        for (const name of [
          'app-icon.svg',
          'bobola-logo.svg',
          ...icons.map((name) => `service-icons/${name}`),
        ]) {
          this.emitFile({
            type: 'asset',
            fileName: name,
            source: await readFile(`public/${name}`),
          });
        }
      },
    },
  ],
  clearScreen: false,
  server: { host: '127.0.0.1', port: 1420, strictPort: true },
});
