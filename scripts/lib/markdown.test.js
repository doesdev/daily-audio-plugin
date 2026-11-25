import test from 'mvt'
import { cellsOf, escapeHtml, flagsWhole, headingsOf, plainText, renderMarkdown, slugify, targetsOf } from './markdown.js'

const html = (md, options) => renderMarkdown(md, options).html

test('slugify makes GitHub\'s anchors', (assert) => {
  const cases = [
    ['Getting started', 'getting-started'],
    ['What `--list` shows', 'what---list-shows'],
    ['Feel, and Aim!', 'feel-and-aim'],
    ['  Spaces  ', 'spaces'],
    ['FX 1 to', 'fx-1-to']
  ]

  for (const [text, want] of cases) assert.is(slugify(text), want, text)
})

test('inlines render to HTML, escaped', (assert) => {
  const cases = [
    ['plain', '<p>plain</p>\n'],
    ['**bold** and *italic* and _also_', '<p><strong>bold</strong> and <em>italic</em> and <em>also</em></p>\n'],
    ['a `<code> & more`', '<p>a <code>&lt;code&gt; &amp; more</code></p>\n'],
    ['1 < 2 & "q"', '<p>1 &lt; 2 &amp; &quot;q&quot;</p>\n'],
    ['snake_case_name stays', '<p>snake_case_name stays</p>\n'],
    ['2 * 3 * 4', '<p>2 * 3 * 4</p>\n'],
    ['\\*not italic\\*', '<p>*not italic*</p>\n'],
    ['**bold with *italic* inside**', '<p><strong>bold with <em>italic</em> inside</strong></p>\n'],
    ['``a ` tick``', '<p><code>a ` tick</code></p>\n'],
    ['line one\nline two', '<p>line one\nline two</p>\n']
  ]

  for (const [md, want] of cases) assert.is(html(md), want, md)
})

test('links: external kept, relative ones resolved into the book', (assert) => {
  const resolveLink = (href) => ({ '02-start.md': '#02-start', '02-start.md#feel': '#02-start--feel' })[href]
  const cases = [
    ['[site](https://example.com)', '<p><a href="https://example.com">site</a><span class="url"> (https://example.com)</span></p>\n'],
    ['[https://example.com](https://example.com)', '<p><a href="https://example.com">https://example.com</a></p>\n'],
    ['[start](02-start.md)', '<p><a class="xref" href="#02-start">start</a></p>\n'],
    ['[Feel](02-start.md#feel)', '<p><a class="xref" href="#02-start--feel">Feel</a></p>\n'],
    ['[gone](nowhere.md)', '<p><span class="xref">gone</span></p>\n'],
    ['[**bold** link](02-start.md)', '<p><a class="xref" href="#02-start"><strong>bold</strong> link</a></p>\n']
  ]

  for (const [md, want] of cases) assert.is(html(md, { resolveLink }), want, md)
})

test('headings are numbered in a chapter and carry anchors', (assert) => {
  const md = '# Getting started\n\nHi.\n\n## Install\n\n### Details\n\n## Feel\n\n## Feel\n'
  const rendered = renderMarkdown(md, { idPrefix: '02-start', chapter: 2 })
  assert.is(rendered.title, 'Getting started')
  assert.deepEqual(rendered.headings, [
    { depth: 1, text: 'Getting started', id: '02-start', number: '2' },
    { depth: 2, text: 'Install', id: '02-start--install', number: '2.1' },
    { depth: 3, text: 'Details', id: '02-start--details' },
    { depth: 2, text: 'Feel', id: '02-start--feel', number: '2.2' },
    { depth: 2, text: 'Feel', id: '02-start--feel-1', number: '2.3' }
  ])
  assert.truthy(rendered.html.startsWith('<h1 id="02-start"><span class="chapter-label">Chapter 2</span>Getting started</h1>'))
  assert.truthy(rendered.html.includes('<h2 id="02-start--install"><span class="secnum">2.1</span>Install</h2>'))

  const bare = renderMarkdown('# Title\n## Part\n')
  assert.is(bare.html, '<h1 id="title">Title</h1>\n<h2 id="part">Part</h2>\n')
})

test('an image alone is a numbered figure, its alt the caption', (assert) => {
  const md = '![The deck](../images/deck.png)\n\nText ![icon](i.png) inline.\n\n![Second <one>](b.png)\n'
  const rendered = renderMarkdown(md, { idPrefix: 'c', chapter: 3, imageSrc: (h) => `file:///x/${h}` })
  assert.is(rendered.figures, 2)
  const cases = [
    '<figure id="c--figure-1"><img src="file:///x/../images/deck.png" alt="The deck"><figcaption><span class="fignum">Figure 3.1.</span> The deck</figcaption></figure>',
    '<p>Text <img class="inline" src="file:///x/i.png" alt="icon"> inline.</p>',
    '<span class="fignum">Figure 3.2.</span> Second &lt;one&gt;</figcaption>'
  ]
  for (const want of cases) assert.truthy(rendered.html.includes(want), want)
})

test('lists, nested by indent, ordered and not', (assert) => {
  const cases = [
    ['- a\n- b\n', '<ul>\n<li>a</li>\n<li>b</li>\n</ul>\n'],
    ['1. one\n2. two\n', '<ol>\n<li>one</li>\n<li>two</li>\n</ol>\n'],
    ['3. three\n4. four\n', '<ol start="3">\n<li>three</li>\n<li>four</li>\n</ol>\n'],
    ['- a\n  - a1\n  - a2\n- b\n', '<ul>\n<li>a<ul>\n<li>a1</li>\n<li>a2</li>\n</ul></li>\n<li>b</li>\n</ul>\n'],
    ['1. one\n   - sub\n2. two\n', '<ol>\n<li>one<ul>\n<li>sub</li>\n</ul></li>\n<li>two</li>\n</ol>\n'],
    ['- a\n  carried on\n- b\n', '<ul>\n<li>a\ncarried on</li>\n<li>b</li>\n</ul>\n'],
    ['- a\nlazy\n', '<ul>\n<li>a\nlazy</li>\n</ul>\n'],
    ['- a\n\n- b\n', '<ul>\n<li><p>a</p></li>\n<li><p>b</p></li>\n</ul>\n'],
    ['- **Bold** lead\n', '<ul>\n<li><strong>Bold</strong> lead</li>\n</ul>\n'],
    ['- a\n\nAfter.\n', '<ul>\n<li>a</li>\n</ul>\n<p>After.</p>\n']
  ]

  for (const [md, want] of cases) assert.is(html(md), want, md)
})

test('tables with a header row and alignment', (assert) => {
  const md = '| Flag | What |\n| :--- | ---: |\n| `--list` | lists `a|b` |\n| `--port NAME` | sends \\| there |\n'
  assert.is(html(md), '<table><thead><tr><th style="text-align:left">Flag</th><th style="text-align:right">What</th></tr></thead>' +
    '<tbody><tr><td style="text-align:left"><code><span class="flag">--list</span></code></td><td style="text-align:right">lists <code>a|b</code></td></tr>' +
    '<tr><td style="text-align:left"><code><span class="flag">--port</span> NAME</code></td><td style="text-align:right">sends | there</td></tr></tbody></table>\n')

  const cases = [
    ['| a | b |', ['a', 'b']],
    ['a | b', ['a', 'b']],
    ['| `x|y` | z |', ['`x|y`', 'z']]
  ]
  for (const [row, want] of cases) assert.deepEqual(cellsOf(row), want, row)
})

test('fenced code is kept as is, escaped', (assert) => {
  const cases = [
    ['```\nwispidi --list\n```\n', '<pre><code>wispidi --list\n</code></pre>\n'.replace('\n</code>', '</code>')],
    ['```powershell\na <b> & c\n\n  d\n```\n', '<pre><code class="language-powershell">a &lt;b&gt; &amp; c\n\n  d</code></pre>\n'],
    ['```\n# not a heading\n```\n', '<pre><code># not a heading</code></pre>\n']
  ]

  for (const [md, want] of cases) assert.is(html(md), want, md)
})

test('quotes: callouts by their label, plain quotes otherwise', (assert) => {
  const cases = [
    ['> **Note.** Keep it low.\n', '<aside class="callout note"><p><strong>Note.</strong> Keep it low.</p>\n</aside>\n'],
    ['> **Tip.** Try Aim.\n> Next line.\n', '<aside class="callout tip"><p><strong>Tip.</strong> Try Aim.\nNext line.</p>\n</aside>\n'],
    ['> **Watch for.** Feedback.\n>\n> - one\n', '<aside class="callout watch"><p><strong>Watch for.</strong> Feedback.</p>\n<ul>\n<li>one</li>\n</ul>\n</aside>\n'],
    ['> Just said.\n', '<blockquote><p>Just said.</p>\n</blockquote>\n']
  ]

  for (const [md, want] of cases) assert.is(html(md), want, md)
})

test('rules, the page break and paragraphs', (assert) => {
  const cases = [
    ['a\n\n---\n\nb\n', '<p>a</p>\n<hr>\n<p>b</p>\n'],
    ['<div class="page-break"></div>\n', '<div class="page-break"></div>\n'],
    ['<b>raw</b>\n', '<p>&lt;b&gt;raw&lt;/b&gt;</p>\n'],
    ['one\ntwo\n\nthree\n', '<p>one\ntwo</p>\n<p>three</p>\n'],
    ['para\n# Heading\n', '<p>para</p>\n<h1 id="heading">Heading</h1>\n']
  ]

  for (const [md, want] of cases) assert.is(html(md), want, md)
})

test('HTML comments are dropped, a line or inline', (assert) => {
  const cases = [
    ['<!-- FIGURE main: The main screen -->\n', ''],
    ['before\n<!-- a note -->\nafter\n', '<p>before</p>\n<p>after</p>\n'],
    ['<!-- one\ntwo -->\n\nkept\n', '<p>kept</p>\n'],
    ['said <!-- not this --> plainly\n', '<p>said  plainly</p>\n'],
    ['`<!-- code keeps it -->`\n', '<p><code>&lt;!-- code keeps it --&gt;</code></p>\n']
  ]

  for (const [md, want] of cases) assert.is(html(md), want, md)
})

test('headingsOf and targetsOf, for the checks', (assert) => {
  const md = '# One\n\nSee [two](02.md#feel) and\n![pic](../images/a.png).\n\n## Two `code`\n\n- [x](https://e.com)\n\n| a |\n|---|\n| [y](y.md) |\n\n> [q](q.md)\n'
  assert.deepEqual(headingsOf(md).map((h) => [h.depth, h.text, h.slug]), [[1, 'One', 'one'], [2, 'Two code', 'two-code']])
  assert.deepEqual(targetsOf(md), [
    { kind: 'link', href: '02.md#feel', line: 3 },
    { kind: 'image', href: '../images/a.png', line: 4 },
    { kind: 'link', href: 'https://e.com', line: 8 },
    { kind: 'link', href: 'y.md', line: 12 },
    { kind: 'link', href: 'q.md', line: 14 }
  ])
})

test('a flag in code stays on one line', (assert) => {
  const cases = [
    ['wispidi --input mic -p x', 'wispidi <span class="flag">--input</span> mic <span class="flag">-p</span> x'],
    ['--list', '<span class="flag">--list</span>'],
    ['a-b and 5 - 3 and --', 'a-b and 5 - 3 and --'],
    ['C:\\x --no-port', 'C:\\x <span class="flag">--no-port</span>']
  ]

  for (const [code, want] of cases) assert.is(flagsWhole(code), want, code)
})

test('plainText and escapeHtml', (assert) => {
  assert.is(plainText('**Feel** and `Aim` [here](x.md)'), 'Feel and Aim here')
  assert.is(escapeHtml('<a href="x">&</a>'), '&lt;a href=&quot;x&quot;&gt;&amp;&lt;/a&gt;')
})
