import { spawnSync } from 'node:child_process'
import { readFileSync } from 'node:fs'
import { homedir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = dirname(fileURLToPath(import.meta.url))
const env = { ...process.env }
const flags = env.CARGO_ENCODED_RUSTFLAGS?.split('\x1f')
  ?? env.RUSTFLAGS?.trim().split(/\s+/).filter(Boolean)
  ?? []
const prefixes = new Map([
  [homedir(), '/build/home'],
  [root, '/build/waypoint'],
  [env.CARGO_HOME || join(homedir(), '.cargo'), '/build/cargo'],
  [env.RUSTUP_HOME || join(homedir(), '.rustup'), '/build/rustup'],
])
for (const [prefix, replacement] of prefixes) {
  flags.push(`--remap-path-prefix=${resolve(prefix)}=${replacement}`)
  flags.push(`--remap-path-prefix=${resolve(prefix).replaceAll('\\', '/')}=${replacement}`)
}
env.CARGO_ENCODED_RUSTFLAGS = flags.join('\x1f')
delete env.RUSTFLAGS

// MSVC also embeds source paths in assertions, including native dependencies.
if (process.platform === 'win32') {
  const nativeFlags = ['/experimental:deterministic', ...[...prefixes]
    .map(([prefix, replacement]) => `/pathmap:${resolve(prefix).replaceAll('\\', '/')}=${replacement}`)]
  env.CL = `${env.CL || ''} ${nativeFlags.map(flag => `"${flag}"`).join(' ')}`.trim()
  // Conan writes these values inside a quoted CMake string.
  env.WAYPOINT_NATIVE_CXXFLAGS = JSON.stringify(nativeFlags.map(flag =>
    /\s/.test(flag) ? `\\"${flag}\\"` : flag))
}

const result = spawnSync(process.execPath, [
  join(root, 'node_modules/@tauri-apps/cli/tauri.js'),
  'build', '--bundles', 'nsis',
], { cwd: root, env, stdio: 'inherit', windowsHide: true })
if (result.error) throw result.error
if (result.status !== 0) process.exit(result.status ?? 1)

const binary = readFileSync(join(root, 'src-tauri/target/release/waypoint.exe'))
for (const prefix of prefixes.keys()) {
  for (const path of [resolve(prefix), resolve(prefix).replaceAll('\\', '/')]) {
    if (['utf8', 'utf16le'].some(encoding => binary.includes(Buffer.from(path, encoding)))) {
      throw new Error('Release contains a local build path; do not distribute it.')
    }
  }
}
console.log('Release privacy check passed: no local build paths in the application.')
