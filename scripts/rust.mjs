import { spawn } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
export function rustEnvironment() {
  const tools = path.join(os.tmpdir(), 'service-workspace-toolchain');
  const localCargo = path.join(
    tools,
    'cargo',
    'bin',
    process.platform === 'win32' ? 'cargo.exe' : 'cargo'
  );
  const local = fs.existsSync(localCargo);
  return {
    executable: local ? localCargo : 'cargo',
    env: {
      ...process.env,
      ...(local
        ? {
            CARGO_HOME: path.join(tools, 'cargo'),
            RUSTUP_HOME: path.join(tools, 'rustup'),
            PATH: `${path.join(tools, 'cargo', 'bin')}${path.delimiter}${process.env.PATH ?? ''}`,
          }
        : {}),
      CARGO_TARGET_DIR:
        process.env.CARGO_TARGET_DIR ?? path.join(os.tmpdir(), 'service-workspace-target'),
      CARGO_BUILD_JOBS: process.env.CARGO_BUILD_JOBS ?? '4',
    },
  };
}
const { executable, env } = rustEnvironment();
const child = spawn(executable, process.argv.slice(2), {
  env,
  stdio: 'inherit',
  windowsHide: false,
});
child.on('error', (error) => {
  console.error(
    `Rust could not start: ${error.message}. Install Rust from https://rustup.rs/ or restore the isolated development toolchain.`
  );
  process.exitCode = 1;
});
child.on('exit', (code) => {
  process.exitCode = code ?? 1;
});
