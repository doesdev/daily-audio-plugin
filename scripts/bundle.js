// `npm run bundle -- PLUGIN [--install]`: builds a plugin in release
// and lays it out the way hosts expect, under target/bundle/PLUGIN:
//
//   NAME.vst3/Contents/ARCH/NAME.vst3   the VST3, a folder
//   NAME.clap                           the CLAP, one file
//
// Both are the same library: it exports both formats' entry points.
// --install also copies them to this user's own plugin folders, which
// needs no admin.

import { execFileSync } from 'node:child_process'
import { cpSync, existsSync, mkdirSync, readFileSync, rmSync } from 'node:fs'
import { homedir } from 'node:os'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'
import { ROOT, displayName, pluginDir, snake } from './lib/plugins.js'

const fail = (code, message) => Object.assign(new Error(message), { code })

// What cargo names the library, and the VST3 folder for this machine.
export const PLATFORMS = {
  win32: { lib: (n) => `${n}.dll`, arch: 'x86_64-win', ext: 'vst3' },
  linux: { lib: (n) => `lib${n}.so`, arch: 'x86_64-linux', ext: 'so' },
  darwin: { lib: (n) => `lib${n}.dylib`, arch: 'MacOS', ext: '' }
}

// Where each format goes inside the bundle folder.
export const layout = (name, platform = process.platform) => {
  const { arch, ext } = PLATFORMS[platform]
  const inner = ext ? `${name}.${ext}` : name
  return {
    vst3: join(`${name}.vst3`, 'Contents', arch, inner),
    clap: `${name}.clap`
  }
}

// This user's plugin folders.
export const userFolders = (platform = process.platform, env = process.env) => {
  if (platform === 'win32') {
    const common = join(env.LOCALAPPDATA ?? '', 'Programs', 'Common')
    return { vst3: join(common, 'VST3'), clap: join(common, 'CLAP') }
  }
  if (platform === 'darwin') {
    const audio = join(homedir(), 'Library', 'Audio', 'Plug-Ins')
    return { vst3: join(audio, 'VST3'), clap: join(audio, 'CLAP') }
  }
  return { vst3: join(homedir(), '.vst3'), clap: join(homedir(), '.clap') }
}

export const bundle = (plugin, { install = false } = {}) => {
  const source = join(pluginDir(plugin), 'src', 'lib.rs')
  if (!existsSync(source)) throw fail('NOPLUGIN', `no plugin named ${plugin}`)
  const name = displayName(readFileSync(source, 'utf8'))
  if (!name) throw fail('NONAME', `${plugin}'s spec has no name`)

  execFileSync(
    process.env.CARGO || 'cargo',
    ['build', '--release', '-p', plugin],
    { cwd: ROOT, stdio: 'inherit' }
  )
  const built = join(
    ROOT, 'target', 'release', PLATFORMS[process.platform].lib(snake(plugin))
  )
  if (!existsSync(built)) throw fail('NOTBUILT', `${built} was not built`)

  const out = join(ROOT, 'target', 'bundle', plugin)
  const paths = layout(name)
  rmSync(out, { recursive: true, force: true })
  mkdirSync(join(out, paths.vst3, '..'), { recursive: true })
  cpSync(built, join(out, paths.vst3))
  cpSync(built, join(out, paths.clap))

  if (install) {
    const folders = userFolders()
    mkdirSync(folders.vst3, { recursive: true })
    mkdirSync(folders.clap, { recursive: true })
    cpSync(join(out, `${name}.vst3`), join(folders.vst3, `${name}.vst3`), {
      recursive: true
    })
    cpSync(join(out, paths.clap), join(folders.clap, paths.clap))
  }
  return { out, name, paths }
}

const main = (args) => {
  const plugin = args.find((a) => !a.startsWith('--'))
  if (!plugin) {
    console.error('bundle: npm run bundle -- PLUGIN [--install]')
    return 1
  }
  try {
    const install = args.includes('--install')
    const { out, name } = bundle(plugin, { install })
    console.log(`bundle: ${name} in ${out}`)
    if (install) console.log('bundle: installed to your plugin folders')
    return 0
  } catch (err) {
    console.error(`bundle: ${err.message}${err.code ? ` (${err.code})` : ''}`)
    return 1
  }
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  process.exitCode = main(process.argv.slice(2))
}
