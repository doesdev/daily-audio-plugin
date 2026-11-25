// A headless Chromium browser (Microsoft Edge on Windows, Chrome or
// Chromium elsewhere) driven over the DevTools protocol with Node's
// own WebSocket: to take the screenshots and print the manuals. No
// Playwright.
//
// The browser is kept off the network: every background service it has
// is off, a proxy that leads nowhere and a resolver that finds nothing
// catch anything left, each page fails any request that would leave
// the machine, and only file URLs are ever loaded. The profile is a
// temp folder, deleted after.

import { spawn } from 'node:child_process'
import { existsSync, mkdtempSync, readFileSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { setTimeout as sleep } from 'node:timers/promises'

const CANDIDATES = [
  'C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe',
  'C:\\Program Files\\Microsoft\\Edge\\Application\\msedge.exe',
  'C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe',
  '/usr/bin/google-chrome',
  '/usr/bin/chromium',
  '/usr/bin/chromium-browser',
  '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'
]

// BROWSER names one; otherwise the first of the usual places.
export const browserPath = (env = process.env, exists = existsSync) =>
  env.BROWSER || CANDIDATES.find((path) => exists(path)) || null

const fail = (code, message) => Object.assign(new Error(message), { code })

const HANDOFF_MS = 5000

export const OFFLINE = [
  '--no-first-run',
  '--no-default-browser-check',
  '--disable-sync',
  '--disable-background-networking',
  '--disable-component-update',
  '--disable-extensions',
  '--disable-default-apps',
  '--disable-domain-reliability',
  '--disable-client-side-phishing-detection',
  '--disable-crash-reporter',
  '--disable-breakpad',
  '--disable-gpu',
  '--hide-scrollbars',
  '--metrics-recording-only',
  '--no-pings',
  '--safebrowsing-disable-auto-update',
  '--no-service-autorun',
  '--password-store=basic',
  '--mute-audio',
  '--proxy-server=http://127.0.0.1:9',
  '--host-resolver-rules=MAP * ~NOTFOUND',
  '--allow-file-access-from-files'
]

// Chromium will not start its sandbox as root, which is what a
// container runs as.
const asRoot = () => process.getuid?.() === 0

const LOCAL = /^(file|data|blob|about):/

// The port and path the browser writes once its DevTools listen.
export const readActivePort = (text) => {
  const [port, path] = text.trim().split(/\r?\n/)
  const ok = /^\d+$/.test(port ?? '') && path?.startsWith('/devtools/')
  return ok ? `ws://127.0.0.1:${port}${path}` : null
}

const waitFor = async (fn, ms, what) => {
  const until = Date.now() + ms
  while (Date.now() < until) {
    const got = fn()
    if (got) return got
    await sleep(50)
  }
  throw fail('BROWSERTIMEOUT', `the browser did not ${what} within ${ms} ms`)
}

// The protocol over one socket: commands by id, events to listeners.
const connect = (url) => new Promise((resolve, reject) => {
  const ws = new WebSocket(url)
  const waiting = new Map()
  const listeners = new Set()
  let next = 1

  ws.onerror = () => reject(
    fail('BROWSERCONNECT', `could not reach the browser's DevTools at ${url}`)
  )
  ws.onclose = () => {
    const gone = fail('BROWSERCLOSED', 'the browser closed its DevTools')
    for (const call of waiting.values()) call.reject(gone)
    waiting.clear()
  }
  ws.onmessage = ({ data }) => {
    const msg = JSON.parse(data)
    if (msg.id === undefined) {
      for (const fn of listeners) fn(msg)
      return
    }
    const call = waiting.get(msg.id)
    if (!call) return

    waiting.delete(msg.id)
    if (msg.error) {
      call.reject(fail('CDP', `${call.method}: ${msg.error.message}`))
    } else {
      call.resolve(msg.result)
    }
  }
  ws.onopen = () => resolve({
    send: (method, params = {}, sessionId) => new Promise((resolve, reject) => {
      const id = next++
      waiting.set(id, { resolve, reject, method })
      const session = sessionId ? { sessionId } : {}
      ws.send(JSON.stringify({ id, method, params, ...session }))
    }),
    on: (fn) => {
      listeners.add(fn)
      return () => listeners.delete(fn)
    },
    close: () => ws.close()
  })
})

// One tab, its commands bound to its session.
const tab = async (cdp) => {
  const { targetId } = await cdp.send(
    'Target.createTarget', { url: 'about:blank' }
  )
  const { sessionId } = await cdp.send(
    'Target.attachToTarget', { targetId, flatten: true }
  )
  const send = (method, params) => cdp.send(method, params, sessionId)
  const errors = []
  const event = (name, ms = 30000) => new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      off()
      reject(fail('BROWSERTIMEOUT', `no ${name} within ${ms} ms`))
    }, ms)
    const off = cdp.on((msg) => {
      if (msg.sessionId !== sessionId || msg.method !== name) return
      clearTimeout(timer)
      off()
      resolve(msg.params)
    })
  })

  await send('Page.enable')
  await send('Runtime.enable')
  // Every request the page makes is let through only if it stays on
  // this machine. What the page throws or logs as an error is kept,
  // so a scene with a broken page fails instead of being shot.
  cdp.on((msg) => {
    if (msg.sessionId !== sessionId) return
    if (msg.method === 'Runtime.exceptionThrown') {
      const details = msg.params.exceptionDetails
      errors.push(details.exception?.description ?? details.text)
    }
    if (msg.method === 'Runtime.consoleAPICalled' &&
      msg.params.type === 'error') {
      errors.push(msg.params.args.map((a) => a.value ?? a.description).join(' '))
    }
    if (msg.method !== 'Fetch.requestPaused') return

    const { requestId, request } = msg.params
    const local = LOCAL.test(request.url)
    const answer = local
      ? send('Fetch.continueRequest', { requestId })
      : send('Fetch.failRequest', { requestId, errorReason: 'BlockedByClient' })
    answer.catch(() => {})
  })
  await send('Fetch.enable', { patterns: [{ urlPattern: '*' }] })

  const evaluate = async (expression) => {
    const { result, exceptionDetails } = await send('Runtime.evaluate', {
      expression, awaitPromise: true, returnByValue: true
    })
    if (exceptionDetails) {
      const { exception, text } = exceptionDetails
      throw fail('PAGEERROR', `${exception?.description ?? text}`)
    }
    return result.value
  }

  return {
    send,
    evaluate,
    errors,
    goto: async (url) => {
      if (!url.startsWith('file:')) {
        throw fail('NOTFILE', `only file URLs are loaded, not ${url}`)
      }

      const loaded = event('Page.loadEventFired')
      const { errorText } = await send('Page.navigate', { url })
      if (errorText) throw fail('NAVIGATE', `${url}: ${errorText}`)
      await loaded
      await evaluate('document.fonts.ready.then(() => true)')
    },
    pdf: async (options = {}) => {
      const { data } = await send('Page.printToPDF', {
        preferCSSPageSize: true,
        printBackground: true,
        displayHeaderFooter: false,
        ...options
      })
      return Buffer.from(data, 'base64')
    },
    close: () => cdp.send('Target.closeTarget', { targetId })
  }
}

// Runs `fn` with a browser to open tabs in, and always closes it and
// deletes its profile, whatever `fn` does.
export const withBrowser = async (fn) => {
  const path = browserPath()
  if (!path) {
    throw fail('BROWSERMISSING',
      'no Edge, Chrome or Chromium found; set BROWSER to one')
  }

  const profile = mkdtempSync(join(tmpdir(), 'kit-browser-'))
  const args = [
    '--headless',
    '--remote-debugging-port=0',
    `--user-data-dir=${profile}`,
    ...OFFLINE,
    ...(asRoot() ? ['--no-sandbox'] : []),
    'about:blank'
  ]
  const child = spawn(path, args, { stdio: 'ignore' })
  let exited = false
  let gone = 0
  child.on('exit', () => { exited = true; gone = Date.now() })
  let cdp = null
  try {
    const portFile = join(profile, 'DevToolsActivePort')
    const url = await waitFor(() => {
      const ready = existsSync(portFile) &&
        readActivePort(readFileSync(portFile, 'utf8'))
      // The launcher can hand off to a new process and exit 0 at once,
      // so its exit only counts if nothing opens soon after.
      if (!ready && exited && Date.now() - gone > HANDOFF_MS) {
        throw fail('BROWSEREXITED', `the browser exited at start (${path})`)
      }
      return ready
    }, 30000, 'open its DevTools')
    cdp = await connect(url)
    return await fn({ open: () => tab(cdp) })
  } finally {
    if (cdp) {
      const closed = cdp.send('Browser.close').catch(() => {})
      await Promise.race([closed, sleep(3000)])
      cdp.close()
    }
    if (!exited) {
      const done = new Promise((resolve) => child.once('exit', resolve))
      await Promise.race([done, sleep(5000)])
      if (!exited) child.kill()
    }
    // The browser's helpers let go of the profile a moment after.
    for (let i = 0; i < 20; i++) {
      try {
        rmSync(profile, { recursive: true, force: true })
        break
      } catch {
        await sleep(250)
      }
    }
  }
}
