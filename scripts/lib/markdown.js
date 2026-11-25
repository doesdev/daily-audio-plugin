// Markdown to HTML for the manual and the cheat sheet, on no
// library: only what the docs use and a printed book needs, written so
// the same file still reads on GitHub.
//
// - headings carry GitHub's anchors, so a link that works there works
//   in the PDF; a chapter's h1 and h2 are numbered ("Chapter 3", "3.2");
// - an image alone in its paragraph is a numbered figure, its alt text
//   the caption;
// - a link to another page of the book becomes a link inside the PDF;
// - a quote opening with **Note.**, **Tip.** or **Watch for.** is a
//   callout of that kind.

const ESCAPES = { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }
export const escapeHtml = (text) => String(text).replace(/[&<>"]/g, (c) => ESCAPES[c])

const EXTERNAL = /^(https?:|mailto:)/
export const isExternal = (href) => EXTERNAL.test(href)

// GitHub's anchor: lower case, punctuation dropped, spaces as hyphens.
export const slugify = (text) => text.toLowerCase().trim()
  .replace(/[^\p{L}\p{N}\p{M}\p{Pc} -]/gu, '')
  .replace(/ /g, '-')

// Unique within one document, as GitHub makes them: a, a-1, a-2.
export const slugger = () => {
  const seen = new Map()
  return (text) => {
    const base = slugify(text)
    const count = seen.get(base)
    if (count === undefined) {
      seen.set(base, 0)
      return base
    }
    seen.set(base, count + 1)
    return `${base}-${count + 1}`
  }
}

// ---- blocks

const FENCE = /^(\s*)(`{3,}|~{3,})\s*([\w-]*)\s*$/
const HEADING = /^(#{1,6})\s+(.*?)\s*#*\s*$/
const MARKER = /^(\s*)([-*+]|(\d{1,9})[.)])(\s+|$)/
const RULE = /^\s{0,3}([-*_])(\s*\1){2,}\s*$/
const TABLE_RULE = /^\s*\|?\s*:?-+:?\s*(\|\s*:?-+:?\s*)*\|?\s*$/
// The one raw HTML the docs may carry: the cheat sheet's page break.
const RAW = /^<div class="page-break"><\/div>$/

const indentOf = (text) => text.match(/^ */)[0].length
const blank = (line) => line === undefined || line.t.trim() === ''

const isTable = (lines, i) => lines[i].t.includes('|') && lines[i + 1] !== undefined &&
  lines[i + 1].t.includes('-') && TABLE_RULE.test(lines[i + 1].t)

const startsBlock = (lines, i) => {
  const t = lines[i].t
  return t.trimStart().startsWith('<!--') || FENCE.test(t) || HEADING.test(t.trimStart()) || /^\s*>/.test(t) || MARKER.test(t) ||
    RULE.test(t) || RAW.test(t.trim()) || isTable(lines, i)
}

// Splits a table row on the pipes outside code spans.
export const cellsOf = (row) => {
  const inner = row.trim().replace(/^\|/, '').replace(/(?<!\\)\|$/, '')
  const cells = []
  let cell = ''
  let code = false
  for (let i = 0; i < inner.length; i++) {
    const c = inner[i]
    if (c === '\\' && inner[i + 1] === '|') {
      cell += '|'
      i++
      continue
    }
    if (c === '`') code = !code
    if (c === '|' && !code) {
      cells.push(cell.trim())
      cell = ''
      continue
    }
    cell += c
  }
  cells.push(cell.trim())
  return cells
}

const alignOf = (cell) => {
  const c = cell.trim()
  if (c.startsWith(':') && c.endsWith(':')) return 'center'
  if (c.endsWith(':')) return 'right'
  return c.startsWith(':') ? 'left' : null
}

const readFence = (lines, i) => {
  const [, indent, fence, lang] = lines[i].t.match(FENCE)
  const body = []
  let j = i + 1
  while (j < lines.length && !lines[j].t.trim().startsWith(fence)) {
    body.push(lines[j].t.slice(Math.min(indent.length, indentOf(lines[j].t))))
    j++
  }
  return [{ type: 'code', lang, text: body.join('\n'), line: lines[i].n }, j + 1]
}

const readQuote = (lines, i) => {
  const inner = []
  let j = i
  while (j < lines.length && /^\s*>/.test(lines[j].t)) {
    inner.push({ t: lines[j].t.replace(/^\s*> ?/, ''), n: lines[j].n })
    j++
  }
  return [{ type: 'quote', blocks: parseLines(inner), line: lines[i].n }, j]
}

const readTable = (lines, i) => {
  const head = cellsOf(lines[i].t)
  const aligns = cellsOf(lines[i + 1].t).map(alignOf)
  const rows = []
  let j = i + 2
  while (j < lines.length && !blank(lines[j]) && lines[j].t.includes('|')) {
    rows.push({ cells: cellsOf(lines[j].t), line: lines[j].n })
    j++
  }
  return [{ type: 'table', head, aligns, rows, line: lines[i].n }, j]
}

const readList = (lines, i) => {
  const first = lines[i].t.match(MARKER)
  const indent = first[1].length
  const ordered = first[3] !== undefined
  const sameKind = (m) => m && m[1].length === indent && (m[3] !== undefined) === ordered
  const items = []
  let j = i
  let loose = false
  while (j < lines.length) {
    const m = lines[j].t.match(MARKER)
    if (!sameKind(m)) break

    const content = indent + m[2].length + Math.max(1, Math.min(m[4].length, 4))
    const body = [{ t: lines[j].t.slice(content), n: lines[j].n }]
    const line = lines[j].n
    j++
    while (j < lines.length) {
      const l = lines[j]
      if (blank(l)) {
        const next = lines[j + 1]
        if (next && !blank(next) && indentOf(next.t) > indent) {
          body.push({ t: '', n: l.n })
          j++
          continue
        }
        break
      }
      const at = indentOf(l.t)
      if (at > indent) {
        body.push({ t: l.t.slice(Math.min(at, content)), n: l.n })
        j++
        continue
      }
      const lazy = !blank(lines[j - 1]) && !startsBlock(lines, j)
      if (!lazy) break
      body.push({ t: l.t.trim(), n: l.n })
      j++
    }
    if (body.some((b) => b.t === '')) loose = true
    items.push({ blocks: parseLines(body), line })
    // A blank line between items makes the list loose.
    if (blank(lines[j]) && sameKind(lines[j + 1]?.t.match(MARKER))) {
      loose = true
      j++
    }
  }
  const start = ordered ? Number(first[3]) : 1
  return [{ type: 'list', ordered, start, loose, items, line: lines[i].n }, j]
}

const readParagraph = (lines, i) => {
  const text = [lines[i].t.trim()]
  let j = i + 1
  while (j < lines.length && !blank(lines[j]) && !startsBlock(lines, j)) {
    text.push(lines[j].t.trim())
    j++
  }
  return [{ type: 'para', text: text.join('\n'), line: lines[i].n }, j]
}

// An HTML comment is a note for the writer, never printed: the line
// after its close.
const skipComment = (lines, i) => {
  let j = i
  while (j < lines.length && !lines[j].t.includes('-->')) j++
  return j + 1
}

const parseLines = (lines) => {
  const blocks = []
  let i = 0
  while (i < lines.length) {
    const t = lines[i].t
    if (t.trim() === '') {
      i++
      continue
    }
    const heading = t.trimStart().match(HEADING)
    let read
    if (t.trimStart().startsWith('<!--')) read = [null, skipComment(lines, i)]
    else if (FENCE.test(t)) read = readFence(lines, i)
    else if (heading && indentOf(t) < 4) read = [{ type: 'heading', depth: heading[1].length, text: heading[2], line: lines[i].n }, i + 1]
    else if (/^\s*>/.test(t)) read = readQuote(lines, i)
    else if (isTable(lines, i)) read = readTable(lines, i)
    else if (RULE.test(t)) read = [{ type: 'rule', line: lines[i].n }, i + 1]
    else if (MARKER.test(t)) read = readList(lines, i)
    else if (RAW.test(t.trim())) read = [{ type: 'raw', text: t.trim(), line: lines[i].n }, i + 1]
    else read = readParagraph(lines, i)
    if (read[0]) blocks.push(read[0])
    i = read[1]
  }
  return blocks
}

export const parse = (markdown) =>
  parseLines(markdown.replace(/\r\n?/g, '\n').split('\n').map((t, i) => ({ t: t.replace(/\t/g, '    '), n: i + 1 })))

// ---- inlines

// A link's or image's parts from `[` at `at`, or null: the text, the
// href and where it ends.
const bracketed = (s, at) => {
  let depth = 0
  let i = at
  for (; i < s.length; i++) {
    if (s[i] === '\\') {
      i++
      continue
    }
    if (s[i] === '[') depth++
    if (s[i] === ']' && --depth === 0) break
  }
  if (i >= s.length || s[i + 1] !== '(') return null

  const close = s.indexOf(')', i + 2)
  if (close < 0) return null

  const target = s.slice(i + 2, close).trim()
  const href = target.match(/^<([^>]*)>|^(\S*)/)
  return { text: s.slice(at + 1, i), href: href[1] ?? href[2], end: close + 1 }
}

const PUNCT = /[\\`*_{}[\]()#+\-.!|<>~"']/

// Walks the inline text once; `on` turns each piece into output, so the
// same walk makes HTML, plain text and the list of targets.
const walk = (s, on, offset = 0) => {
  let out = ''
  let i = 0
  while (i < s.length) {
    const c = s[i]
    if (c === '\\' && PUNCT.test(s[i + 1] ?? '')) {
      out += on.text(s[i + 1])
      i += 2
      continue
    }
    if (c === '\n') {
      out += on.text('\n')
      i++
      continue
    }
    if (s.startsWith('<!--', i)) {
      const close = s.indexOf('-->', i + 4)
      i = close < 0 ? s.length : close + 3
      continue
    }
    if (c === '`') {
      const run = s.slice(i).match(/^`+/)[0]
      const close = s.indexOf(run, i + run.length)
      if (close > 0) {
        out += on.code(s.slice(i + run.length, close).replace(/\n/g, ' ').replace(/^ (.+) $/, '$1'))
        i = close + run.length
        continue
      }
    }
    if ((c === '!' && s[i + 1] === '[') || c === '[') {
      const image = c === '!'
      const part = bracketed(s, image ? i + 1 : i)
      if (part) {
        out += image ? on.image(part.text, part.href, offset + i) : on.link(walk(part.text, on, offset + i + 1), part.href, offset + i, part.text)
        i = part.end
        continue
      }
    }
    if (c === '*' || c === '_') {
      const strong = s[i + 1] === c
      const mark = strong ? c + c : c
      const before = s[i - 1] ?? ' '
      const intraword = c === '_' && /[\p{L}\p{N}]/u.test(before)
      const close = intraword ? -1 : findClose(s, mark, i + mark.length)
      if (close > i + mark.length) {
        const inner = walk(s.slice(i + mark.length, close), on, offset + i + mark.length)
        out += strong ? on.strong(inner) : on.em(inner)
        i = close + mark.length
        continue
      }
    }
    out += on.text(c)
    i++
  }
  return out
}

// Where `mark` closes, not at a space, nor a single mark inside a double.
const findClose = (s, mark, from) => {
  if (/\s/.test(s[from] ?? ' ')) return -1

  for (let i = from; i < s.length; i++) {
    if (s[i] === '\\') {
      i++
      continue
    }
    if (s[i] === '`') {
      const run = s.slice(i).match(/^`+/)[0]
      const close = s.indexOf(run, i + run.length)
      if (close > 0) i = close + run.length - 1
      continue
    }
    if (!s.startsWith(mark, i) || /\s/.test(s[i - 1])) continue
    if (mark.length === 1 && s[i + 1] === mark) {
      const end = findClose(s, mark + mark, i + 2)
      if (end > 0) {
        i = end + 1
        continue
      }
    }
    if (mark === '_' && /[\p{L}\p{N}]/u.test(s[i + 1] ?? ' ')) continue
    return i
  }
  return -1
}

const TEXT = {
  text: (c) => c === '\n' ? ' ' : c,
  code: (t) => t,
  image: (alt) => alt,
  link: (inner) => inner,
  strong: (inner) => inner,
  em: (inner) => inner
}

// The plain text of inline Markdown: what an anchor is made from.
export const plainText = (s) => walk(s, TEXT)

// ---- the document's headings and targets, for the checks

const eachBlock = (blocks, fn) => {
  for (const b of blocks) {
    fn(b)
    if (b.type === 'quote') eachBlock(b.blocks, fn)
    if (b.type === 'list') b.items.forEach((item) => eachBlock(item.blocks, fn))
  }
}

export const headingsOf = (markdown) => {
  const slug = slugger()
  const out = []
  eachBlock(parse(markdown), (b) => {
    if (b.type !== 'heading') return

    const text = plainText(b.text)
    out.push({ depth: b.depth, text, slug: slug(text), line: b.line })
  })
  return out
}

// Every link and image target, with the line it is on.
export const targetsOf = (markdown) => {
  const out = []
  const collect = (text, line) => {
    const at = (offset) => line + (text.slice(0, offset).match(/\n/g)?.length ?? 0)
    walk(text, {
      ...TEXT,
      image: (alt, href, offset) => {
        out.push({ kind: 'image', href, line: at(offset) })
        return ''
      },
      link: (inner, href, offset) => {
        out.push({ kind: 'link', href, line: at(offset) })
        return ''
      }
    })
  }
  eachBlock(parse(markdown), (b) => {
    if (b.type === 'para' || b.type === 'heading') collect(b.text, b.line)
    if (b.type === 'table') {
      b.head.forEach((c) => collect(c, b.line))
      b.rows.forEach((r) => r.cells.forEach((c) => collect(c, r.line)))
    }
  })
  return out
}

// ---- HTML

// A flag in inline code kept on one line: a line may break after a
// hyphen, and "-" then "-name" reads as two things.
export const flagsWhole = (html) => html.replace(/(^|\s)(--?[A-Za-z][\w-]*)/g, '$1<span class="flag">$2</span>')

const CALLOUTS = [['Note.', 'note'], ['Tip.', 'tip'], ['Watch for.', 'watch']]

const calloutOf = (blocks) => {
  const first = blocks[0]
  if (first?.type !== 'para') return null

  return CALLOUTS.find(([label]) => first.text.startsWith(`**${label}**`))?.[1] ?? null
}

// Renders Markdown. `idPrefix` starts every anchor, so two chapters can
// both have a Safety section; `chapter` numbers the headings and
// figures; `resolveLink(href)` is a relative link's in-book "#id", or
// undefined to print its text alone; `imageSrc(href)` is where an
// image loads from; `figureWidth(href)` is a figure's width in CSS
// pixels, or undefined for the page's own.
export const renderMarkdown = (markdown, options = {}) => {
  const {
    idPrefix = '',
    chapter,
    resolveLink = (href) => href,
    imageSrc = (href) => href,
    figureWidth = () => undefined
  } = options
  const slug = slugger()
  const headings = []
  let title = ''
  let section = 0
  let figures = 0

  const HTML = {
    text: (c) => escapeHtml(c),
    code: (t) => `<code>${flagsWhole(escapeHtml(t))}</code>`,
    image: (alt, href) => `<img class="inline" src="${escapeHtml(imageSrc(href))}" alt="${escapeHtml(alt)}">`,
    link: (inner, href, _, raw) => {
      if (isExternal(href)) {
        const shown = plainText(raw)
        const bare = href.replace(/^mailto:/, '')
        const url = shown === href || shown === bare ? '' : `<span class="url"> (${escapeHtml(bare)})</span>`
        return `<a href="${escapeHtml(href)}">${inner}</a>${url}`
      }
      const target = resolveLink(href)
      return target === undefined ? `<span class="xref">${inner}</span>` : `<a class="xref" href="${escapeHtml(target)}">${inner}</a>`
    },
    strong: (inner) => `<strong>${inner}</strong>`,
    em: (inner) => `<em>${inner}</em>`
  }
  const inline = (text) => walk(text, HTML)
  const anchor = (s, depth) => {
    if (!idPrefix) return s
    return depth === 1 ? idPrefix : `${idPrefix}--${s}`
  }

  const figure = (text) => {
    const m = text.match(/^!\[([^\]]*)\]\(\s*<?([^)\s>]*)>?[^)]*\)$/)
    if (!m) return null

    figures++
    const number = chapter === undefined ? `${figures}` : `${chapter}.${figures}`
    const id = `${idPrefix ? `${idPrefix}--` : ''}figure-${figures}`
    const alt = plainText(m[1])
    const width = figureWidth(m[2])
    const style = width === undefined ? '' : ` style="width: ${Math.round(width)}px"`
    return `<figure id="${id}"><img src="${escapeHtml(imageSrc(m[2]))}" alt="${escapeHtml(alt)}"${style}><figcaption><span class="fignum">Figure ${number}.</span> ${inline(m[1])}</figcaption></figure>\n`
  }

  const heading = (b) => {
    const text = plainText(b.text)
    const id = anchor(slug(text), b.depth)
    const info = { depth: b.depth, text, id }
    let label = ''
    if (b.depth === 1) {
      if (!title) title = text
      if (chapter !== undefined) {
        info.number = String(chapter)
        label = `<span class="chapter-label">Chapter ${chapter}</span>`
      }
    } else if (b.depth === 2 && chapter !== undefined) {
      section++
      info.number = `${chapter}.${section}`
      label = `<span class="secnum">${info.number}</span>`
    }
    headings.push(info)
    return `<h${b.depth} id="${escapeHtml(id)}">${label}${inline(b.text)}</h${b.depth}>\n`
  }

  const table = (b) => {
    const cell = (tag, text, i) => {
      const align = b.aligns[i] ? ` style="text-align:${b.aligns[i]}"` : ''
      return `<${tag}${align}>${inline(text)}</${tag}>`
    }
    const head = `<thead><tr>${b.head.map((c, i) => cell('th', c, i)).join('')}</tr></thead>`
    const rows = b.rows.map((r) => `<tr>${b.head.map((_, i) => cell('td', r.cells[i] ?? '', i)).join('')}</tr>`)
    return `<table>${head}<tbody>${rows.join('')}</tbody></table>\n`
  }

  const list = (b) => {
    const tag = b.ordered ? 'ol' : 'ul'
    const start = b.ordered && b.start !== 1 ? ` start="${b.start}"` : ''
    const item = (it) => {
      const tight = !b.loose && it.blocks.every((x, i) => x.type !== 'para' || i === 0)
      const inner = it.blocks.map((x, i) => tight && i === 0 && x.type === 'para' ? inline(x.text) : render(x)).join('')
      return `<li>${inner.replace(/\n$/, '')}</li>\n`
    }
    return `<${tag}${start}>\n${b.items.map(item).join('')}</${tag}>\n`
  }

  const quote = (b) => {
    const inner = b.blocks.map(render).join('')
    const kind = calloutOf(b.blocks)
    return kind ? `<aside class="callout ${kind}">${inner}</aside>\n` : `<blockquote>${inner}</blockquote>\n`
  }

  const RENDER = {
    heading,
    para: (b) => figure(b.text) ?? `<p>${inline(b.text)}</p>\n`,
    code: (b) => `<pre><code${b.lang ? ` class="language-${escapeHtml(b.lang)}"` : ''}>${escapeHtml(b.text)}</code></pre>\n`,
    quote,
    list,
    table,
    rule: () => '<hr>\n',
    raw: (b) => `${b.text}\n`
  }
  const render = (b) => RENDER[b.type](b)

  const html = parse(markdown).map(render).join('')
  return { html, title, headings, figures }
}
