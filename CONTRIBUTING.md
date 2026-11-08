# Contributing to Bobola

Thanks for helping make a calmer Windows workspace. Small, focused fixes, clear bug reports, accessibility improvements, and reproducible compatibility results are welcome.

## Start here

- Check existing issues before opening a new one.
- For substantial behavior or architecture changes, describe the problem in an issue first so the approach can be discussed.
- Include the Bobola version, Windows version, relevant service, and reproducible steps. Use test accounts and remove private information from logs and screenshots.

## Development

Use Windows x64, Node.js 22+, stable Rust with the MSVC target, Windows C++ Build Tools, and WebView2.

```powershell
npm ci
npm run build
npm test
npm run desktop
```

`npm run dev` provides an interface preview; native features must be tested in the desktop app. Browser-preview storage is separate from native profiles.

## Before a pull request

```powershell
npm run build
npm test
node scripts/rust.mjs fmt --all -- --check
node scripts/rust.mjs clippy --workspace --all-targets --all-features -- -D warnings
```

For interface changes, run `node scripts/launcher-ui-check.mjs` after building; it needs Microsoft Edge. For native behavior changes, build the debug desktop app and run the appropriate native smoke checks. Describe what you tested and any service-specific gaps.

Keep changes focused and follow existing naming and formatting. Add meaningful regression coverage for behavior fixes. Preserve pinned artwork bytes and update provenance deliberately when assets change. Never include browser profiles, sign-in data, credentials, or local build output.

Performance claims need the full owned process tree, comparable workloads, repeated runs, and documented units. Use [the benchmark methodology](docs/BENCHMARK_ANALYSIS.md).

## Pull request description

Explain the problem, the resulting behavior, and validation. Include before/after screenshots for visible changes. Contributions are provided under the project's Apache-2.0 license; retain applicable third-party notices.
