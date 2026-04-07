#!/usr/bin/env node
/**
 * md-to-docx — Convert a Markdown file to a styled .docx using docx-js.
 *
 * Usage:
 *   NODE_PATH=$(npm root -g) node build_docx.js <input.md> <output.docx> [options]
 *
 * Run with --help for the full option list. All options have defaults that
 * reproduce the reference style exactly.
 */
'use strict';

const fs = require('fs');
const path = require('path');

let marked, docx;
try {
  marked = require('marked').marked;
  docx = require('docx');
} catch (e) {
  console.error('Error: missing dependency. Install with:');
  console.error('  npm install -g docx marked');
  console.error('Then run with: NODE_PATH=$(npm root -g) node build_docx.js ...');
  process.exit(1);
}

const {
  Document, Packer, Paragraph, TextRun, Table, TableRow, TableCell,
  ImageRun, AlignmentType, LevelFormat, ExternalHyperlink, HeadingLevel,
  BorderStyle, WidthType, ShadingType, FootnoteReferenceRun,
} = docx;

// ---------- argv parsing ----------
function parseArgs(argv) {
  const positional = [];
  const cliOpts = {};
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === '--help' || a === '-h') { cliOpts.help = true; continue; }
    if (a.startsWith('--')) {
      const key = a.slice(2);
      const val = argv[i + 1];
      if (val === undefined || val.startsWith('--')) {
        console.error(`Error: --${key} requires a value`);
        process.exit(1);
      }
      cliOpts[key] = val;
      i++;
    } else {
      positional.push(a);
    }
  }
  return { positional, cliOpts };
}

function parseNumber(val, def, name) {
  if (val === undefined) return def;
  const n = Number(val);
  if (!Number.isFinite(n)) {
    console.error(`Error: --${name} must be a number, got "${val}"`);
    process.exit(1);
  }
  return n;
}

function parseColor(val, def, name) {
  if (val === undefined) return def;
  const c = String(val).replace(/^#/, '');
  if (!/^[0-9A-Fa-f]{6}$/.test(c)) {
    console.error(`Error: --${name} must be a 6-digit hex color, got "${val}"`);
    process.exit(1);
  }
  return c.toUpperCase();
}

const { positional, cliOpts } = parseArgs(process.argv.slice(2));

if (cliOpts.help || positional.length < 2) {
  console.log(`md-to-docx — convert Markdown to a styled .docx

Usage:
  NODE_PATH=$(npm root -g) node build_docx.js <input.md> <output.docx> [options]

Options:
  --font <name>            Body font (default: "Yu Gothic")
  --mono <name>            Monospace font (default: "Menlo")
  --page <a4|letter>       Page size (default: a4)
  --margin <inches>        Page margin in inches (default: 1)
  --title-color <hex>      H1 color, no # (default: 1F3864)
  --accent <hex>           H2/H3 color, no # (default: 2E74B5)
  --max-image-px <n>       Max image width in pixels (default: 580)
  --resource-path <p1:p2>  Extra image search dirs (default: md file dir)
  -h, --help               Show this help`);
  process.exit(cliOpts.help ? 0 : 1);
}

const MD_PATH = positional[0];
const OUT_PATH = positional[1];
const mdDir = path.dirname(path.resolve(MD_PATH));

const PAGE_SIZES = {
  a4: { w: 11906, h: 16838 },
  letter: { w: 12240, h: 15840 },
};

const FONT_NAME = cliOpts.font || 'Yu Gothic';
const MONO_NAME = cliOpts.mono || 'Menlo';
const PAGE = (cliOpts.page || 'a4').toLowerCase();
if (!(PAGE in PAGE_SIZES)) {
  console.error(`Error: --page must be one of ${Object.keys(PAGE_SIZES).join(', ')}, got "${cliOpts.page}"`);
  process.exit(1);
}
const MARGIN_IN = parseNumber(cliOpts.margin, 1, 'margin');
const TITLE_COLOR = parseColor(cliOpts['title-color'], '1F3864', 'title-color');
const ACCENT_COLOR = parseColor(cliOpts.accent, '2E74B5', 'accent');
const MAX_IMG_PX = Math.floor(parseNumber(cliOpts['max-image-px'], 580, 'max-image-px'));
const EXTRA_PATHS = String(cliOpts['resource-path'] || '').split(':').filter(Boolean);

const FONT = { name: FONT_NAME, hint: 'eastAsia' };
const MONO = { name: MONO_NAME };

const pageSize = PAGE_SIZES[PAGE];
const MARGIN_DXA = Math.round(MARGIN_IN * 1440);
const CONTENT_WIDTH = pageSize.w - 2 * MARGIN_DXA;

// ---------- read + pre-process ----------
let mdRaw = fs.readFileSync(MD_PATH, 'utf8');

// Protect code blocks (fenced + inline) from math/footnote substitutions.
// We swap them out for placeholders before any other replacement, then
// restore after marked has lexed (text tokens are restored too).
const codeStash = [];
function stash(s) {
  const i = codeStash.length;
  codeStash.push(s);
  return `\uE200${i}\uE201`;
}
mdRaw = mdRaw.replace(/```[\s\S]*?```/g, (m) => stash(m));
mdRaw = mdRaw.replace(/`[^`\n]+`/g, (m) => stash(m));

// footnotes
const footnoteDefs = {};
mdRaw = mdRaw.replace(/^\[\^([^\]]+)\]:\s*(.+)$/gm, (_, id, txt) => {
  footnoteDefs[id] = txt.trim();
  return '';
});
const fnIds = Object.keys(footnoteDefs);
const fnIndex = {};
fnIds.forEach((id, i) => { fnIndex[id] = i + 1; });
mdRaw = mdRaw.replace(/\[\^([^\]]+)\]/g, (_, id) => `\uE000FN${fnIndex[id] || id}\uE001`);

// math
function texToText(tex) {
  return tex
    .replace(/\\text\{([^}]+)\}/g, '$1')
    .replace(/\\mathrm\{([^}]+)\}/g, '$1')
    .replace(/\\frac\{([^}]+)\}\{([^}]+)\}/g, '($1) ÷ ($2)')
    .replace(/\\geq/g, '≥').replace(/\\leq/g, '≤')
    .replace(/\\gg/g, '≫').replace(/\\ll/g, '≪')
    .replace(/\\times/g, '×').replace(/\\cdot/g, '·')
    .replace(/\\le\b/g, '≤').replace(/\\ge\b/g, '≥')
    .replace(/\\sum\b/g, '∑').replace(/\\prod\b/g, '∏')
    .replace(/\\sqrt\{([^}]+)\}/g, '√($1)')
    .replace(/\\pm/g, '±').replace(/\\mp/g, '∓')
    .replace(/\\approx/g, '≈').replace(/\\neq/g, '≠')
    .replace(/\\infty/g, '∞').replace(/\\to/g, '→')
    .replace(/_(\{[^}]+\})/g, (_, g) => g.replace(/[{}]/g, ''))
    .replace(/_(\w)/g, '_$1')
    .replace(/\^(\{[^}]+\})/g, (_, g) => g.replace(/[{}]/g, ''))
    .replace(/\^(\w)/g, '^$1')
    .replace(/\{([^}]+)\}/g, '$1')
    .replace(/\\,/g, ' ').replace(/\\;/g, ' ')
    .trim();
}
mdRaw = mdRaw.replace(/\$\$([\s\S]*?)\$\$/g, (_, tex) => '\n\n\uE100' + texToText(tex) + '\uE101\n\n');
mdRaw = mdRaw.replace(/\$([^$\n]+)\$/g, (_, tex) => '\uE102' + texToText(tex) + '\uE103');

// Restore code blocks now that math/footnote substitutions are done.
mdRaw = mdRaw.replace(/\uE200(\d+)\uE201/g, (_, i) => codeStash[+i]);

marked.use({ gfm: true });
const tokens = marked.lexer(mdRaw);

// ---------- helpers ----------
function displayWidth(s) {
  let w = 0;
  for (const ch of String(s || '')) {
    const code = ch.codePointAt(0);
    w += (code > 0x2E80 && !(code >= 0xE000 && code <= 0xE1FF)) ? 2.0 : 1.0;
  }
  return w;
}

function decodeHtml(s) {
  return s
    .replace(/&amp;/g, '&').replace(/&lt;/g, '<').replace(/&gt;/g, '>')
    .replace(/&quot;/g, '"').replace(/&#39;/g, "'")
    .replace(/&#x([0-9a-fA-F]+);/g, (_, h) => String.fromCodePoint(parseInt(h, 16)))
    .replace(/&#(\d+);/g, (_, d) => String.fromCodePoint(parseInt(d, 10)));
}

function resolveImage(href) {
  const candidates = [path.resolve(mdDir, href), ...EXTRA_PATHS.map(p => path.resolve(p, href))];
  for (const c of candidates) if (fs.existsSync(c)) return c;
  return null;
}

// Detect image type and read dimensions from file headers.
// Returns null for unsupported formats (caller should warn / skip).
function detectImage(buf) {
  // PNG: 89 50 4E 47 ... IHDR width/height at offset 16/20 (big-endian u32)
  if (buf.length >= 24 && buf[0] === 0x89 && buf[1] === 0x50 && buf[2] === 0x4E && buf[3] === 0x47) {
    return { type: 'png', w: buf.readUInt32BE(16), h: buf.readUInt32BE(20) };
  }
  // JPEG: FF D8 ... scan SOFn marker for dimensions
  if (buf.length >= 4 && buf[0] === 0xFF && buf[1] === 0xD8) {
    let i = 2;
    while (i < buf.length - 9) {
      if (buf[i] !== 0xFF) break;
      const marker = buf[i + 1];
      // SOF0..SOF3, SOF5..SOF7, SOF9..SOF11, SOF13..SOF15
      if ((marker >= 0xC0 && marker <= 0xC3) || (marker >= 0xC5 && marker <= 0xC7) ||
          (marker >= 0xC9 && marker <= 0xCB) || (marker >= 0xCD && marker <= 0xCF)) {
        return { type: 'jpg', h: buf.readUInt16BE(i + 5), w: buf.readUInt16BE(i + 7) };
      }
      const segLen = buf.readUInt16BE(i + 2);
      i += 2 + segLen;
    }
    return { type: 'jpg', w: 600, h: 400 };
  }
  // GIF: "GIF87a" or "GIF89a", width/height at offset 6/8 (little-endian u16)
  if (buf.length >= 10 && buf[0] === 0x47 && buf[1] === 0x49 && buf[2] === 0x46) {
    return { type: 'gif', w: buf.readUInt16LE(6), h: buf.readUInt16LE(8) };
  }
  return null;
}

// Split text on placeholder markers (footnote ref / math) into runs.
// `runOpts` carries TextRun formatting (bold/italic/etc.) — distinct from
// the top-level `cliOpts`.
function expandText(text, runOpts = {}) {
  const out = [];
  const re = /\uE000FN(\d+)\uE001|\uE100([^\uE101]*)\uE101|\uE102([^\uE103]*)\uE103/g;
  let last = 0, m;
  while ((m = re.exec(text)) !== null) {
    if (m.index > last) {
      out.push(new TextRun({ text: text.slice(last, m.index), font: FONT, ...runOpts }));
    }
    if (m[1] !== undefined) {
      out.push(new FootnoteReferenceRun(parseInt(m[1], 10)));
    } else if (m[2] !== undefined) {
      out.push(new TextRun({ text: m[2], font: FONT, italics: true, ...runOpts }));
    } else if (m[3] !== undefined) {
      out.push(new TextRun({ text: m[3], font: FONT, italics: true, ...runOpts }));
    }
    last = m.index + m[0].length;
  }
  if (last < text.length) {
    out.push(new TextRun({ text: text.slice(last), font: FONT, ...runOpts }));
  }
  return out;
}

function inlineToRuns(toks, runOpts = {}) {
  const runs = [];
  for (const t of toks || []) {
    switch (t.type) {
      case 'text':
        if (t.tokens && t.tokens.length) runs.push(...inlineToRuns(t.tokens, runOpts));
        else runs.push(...expandText(decodeHtml(t.text), runOpts));
        break;
      case 'escape':
        runs.push(new TextRun({ text: t.text, font: FONT, ...runOpts }));
        break;
      case 'strong':
        runs.push(...inlineToRuns(t.tokens, { ...runOpts, bold: true }));
        break;
      case 'em':
        runs.push(...inlineToRuns(t.tokens, { ...runOpts, italics: true }));
        break;
      case 'codespan':
        runs.push(new TextRun({
          text: decodeHtml(t.text), font: MONO, ...runOpts,
          shading: { fill: 'F2F2F2', type: ShadingType.CLEAR },
        }));
        break;
      case 'link': {
        const childRuns = inlineToRuns(t.tokens, { ...runOpts, color: ACCENT_COLOR, underline: {} });
        runs.push(new ExternalHyperlink({ children: childRuns, link: t.href }));
        break;
      }
      case 'del':
        runs.push(...inlineToRuns(t.tokens, { ...runOpts, strike: true }));
        break;
      case 'br':
        runs.push(new TextRun({ text: '', break: 1, font: FONT }));
        break;
      case 'html':
        break;
      default:
        if (t.text) runs.push(...expandText(decodeHtml(t.text), runOpts));
    }
  }
  return runs;
}

// ---------- table builder ----------
const cellBorder = { style: BorderStyle.SINGLE, size: 4, color: 'BFBFBF' };
const cellBorders = { top: cellBorder, bottom: cellBorder, left: cellBorder, right: cellBorder };

function cellPlainText(c) {
  function walk(toks) {
    let s = '';
    for (const t of toks || []) {
      if (t.text) s += t.text;
      if (t.tokens) s += walk(t.tokens);
    }
    return s;
  }
  return walk(c.tokens || []) || c.text || '';
}

function buildTable(token) {
  const numCols = token.header.length;
  const colW = new Array(numCols).fill(0);
  token.header.forEach((c, i) => {
    colW[i] = Math.max(colW[i], displayWidth(cellPlainText(c)));
  });
  token.rows.forEach(row => {
    row.forEach((c, i) => {
      // Cap body cells at 40 display-units so a single huge cell does not
      // monopolize the column; wider content wraps over multiple lines.
      const w = displayWidth(cellPlainText(c));
      colW[i] = Math.max(colW[i], Math.min(w, 40));
    });
  });
  // Floor each column at a small minimum so empty headers do not collapse.
  for (let i = 0; i < numCols; i++) colW[i] = Math.max(colW[i], 5);

  // Distribute the content width proportionally.
  const total = colW.reduce((a, b) => a + b, 0);
  const dxa = colW.map(w => Math.floor((w / total) * CONTENT_WIDTH));
  const diff = CONTENT_WIDTH - dxa.reduce((a, b) => a + b, 0);
  dxa[dxa.length - 1] += diff;

  function makeCell(cellTok, i, isHeader) {
    return new TableCell({
      borders: cellBorders,
      width: { size: dxa[i], type: WidthType.DXA },
      shading: isHeader ? { fill: 'DEEAF6', type: ShadingType.CLEAR } : undefined,
      margins: { top: 100, bottom: 100, left: 120, right: 120 },
      children: [new Paragraph({
        spacing: { before: 0, after: 0, line: 280 },
        children: inlineToRuns(cellTok.tokens, isHeader ? { bold: true } : {}),
      })],
    });
  }

  const headerRow = new TableRow({
    tableHeader: true,
    children: token.header.map((c, i) => makeCell(c, i, true)),
  });
  const bodyRows = token.rows.map(row => new TableRow({
    children: row.map((c, i) => makeCell(c, i, false)),
  }));

  return new Table({
    width: { size: CONTENT_WIDTH, type: WidthType.DXA },
    columnWidths: dxa,
    rows: [headerRow, ...bodyRows],
  });
}

// ---------- list item ----------
function listItemChildren(item, ref, level) {
  const out = [];
  let inlineBuffer = [];
  const flushInline = () => {
    if (inlineBuffer.length) {
      out.push(new Paragraph({
        numbering: { reference: ref, level },
        spacing: { before: 40, after: 40, line: 300 },
        children: inlineBuffer,
      }));
      inlineBuffer = [];
    }
  };
  for (const t of item.tokens || []) {
    if (t.type === 'text') {
      if (t.tokens) inlineBuffer.push(...inlineToRuns(t.tokens));
      else inlineBuffer.push(...expandText(decodeHtml(t.text), {}));
    } else if (t.type === 'paragraph') {
      flushInline();
      out.push(new Paragraph({
        numbering: { reference: ref, level },
        spacing: { before: 40, after: 40, line: 300 },
        children: inlineToRuns(t.tokens),
      }));
    } else if (t.type === 'list') {
      flushInline();
      for (const sub of t.items) {
        out.push(...listItemChildren(sub, t.ordered ? 'numbers' : 'bullets', level + 1));
      }
    } else if (t.type === 'code') {
      flushInline();
      out.push(...codeBlock(t));
    }
  }
  flushInline();
  return out;
}

function codeBlock(t) {
  const lines = t.text.split('\n');
  return lines.map((line, i) => new Paragraph({
    shading: { fill: 'F5F5F5', type: ShadingType.CLEAR },
    spacing: { before: i === 0 ? 120 : 0, after: i === lines.length - 1 ? 120 : 0, line: 260 },
    indent: { left: 200, right: 200 },
    border: i === 0 ? { top: { style: BorderStyle.SINGLE, size: 4, color: 'D0D0D0', space: 4 } } : undefined,
    children: [new TextRun({ text: line || ' ', font: MONO, size: 18 })],
  }));
}

// ---------- main walker ----------
function walk(toks) {
  const els = [];
  for (const t of toks) {
    switch (t.type) {
      case 'heading': {
        const level = Math.min(Math.max(t.depth, 1), 5);
        const map = [HeadingLevel.HEADING_1, HeadingLevel.HEADING_2, HeadingLevel.HEADING_3, HeadingLevel.HEADING_4, HeadingLevel.HEADING_5];
        els.push(new Paragraph({
          heading: map[level - 1],
          children: inlineToRuns(t.tokens),
        }));
        break;
      }
      case 'paragraph': {
        if (t.tokens.length === 1 && t.tokens[0].type === 'image') {
          const img = t.tokens[0];
          const imgPath = resolveImage(img.href);
          if (!imgPath) {
            console.warn(`Warning: image not found: ${img.href}`);
            break;
          }
          const data = fs.readFileSync(imgPath);
          const info = detectImage(data);
          if (!info) {
            console.warn(`Warning: unsupported image format (PNG/JPEG/GIF only): ${img.href}`);
            break;
          }
          let dispW = info.w, dispH = info.h;
          if (dispW > MAX_IMG_PX) { dispH = Math.floor(dispH * MAX_IMG_PX / dispW); dispW = MAX_IMG_PX; }
          els.push(new Paragraph({
            alignment: AlignmentType.CENTER,
            spacing: { before: 200, after: 100 },
            children: [new ImageRun({
              type: info.type, data,
              transformation: { width: dispW, height: dispH },
              altText: { title: img.text || 'image', description: img.text || 'image', name: 'image' },
            })],
          }));
          if (img.text) {
            els.push(new Paragraph({
              alignment: AlignmentType.CENTER,
              spacing: { before: 0, after: 200 },
              children: [new TextRun({ text: img.text, italics: true, size: 20, font: FONT, color: '595959' })],
            }));
          }
          break;
        }
        const ptext = (t.text || '');
        if (/^\uE100[^\uE101]*\uE101$/.test(ptext.trim())) {
          const inner = ptext.trim().slice(1, -1);
          els.push(new Paragraph({
            alignment: AlignmentType.CENTER,
            spacing: { before: 160, after: 160 },
            children: [new TextRun({ text: inner, italics: true, font: FONT, size: 26 })],
          }));
          break;
        }
        els.push(new Paragraph({
          spacing: { before: 80, after: 80, line: 320 },
          children: inlineToRuns(t.tokens),
        }));
        break;
      }
      case 'list': {
        const ref = t.ordered ? 'numbers' : 'bullets';
        for (const item of t.items) {
          els.push(...listItemChildren(item, ref, 0));
        }
        break;
      }
      case 'code':
        els.push(...codeBlock(t));
        break;
      case 'table':
        els.push(buildTable(t));
        els.push(new Paragraph({ spacing: { before: 0, after: 120 }, children: [] }));
        break;
      case 'hr':
        els.push(new Paragraph({
          spacing: { before: 200, after: 200 },
          border: { bottom: { style: BorderStyle.SINGLE, size: 6, color: 'BFBFBF', space: 1 } },
          children: [],
        }));
        break;
      case 'blockquote':
        for (const inner of t.tokens || []) {
          if (inner.type === 'paragraph') {
            els.push(new Paragraph({
              indent: { left: 360 },
              border: { left: { style: BorderStyle.SINGLE, size: 16, color: 'BFBFBF', space: 8 } },
              spacing: { before: 80, after: 80 },
              children: inlineToRuns(inner.tokens),
            }));
          }
        }
        break;
      case 'space':
      case 'def':
      case 'html':
        break;
      default:
        if (t.tokens) els.push(...walk(t.tokens));
    }
  }
  return els;
}

const bodyChildren = walk(tokens);

// ---------- footnotes config ----------
// Walk every block-level token of the footnote body and collect inline runs,
// so footnotes that start with a list / code / blockquote do not silently
// drop content.
const footnotesConfig = {};
fnIds.forEach((id) => {
  const idx = fnIndex[id];
  const blockToks = marked.lexer(footnoteDefs[id]);
  const runs = [];
  for (const bt of blockToks) {
    if (bt.tokens) runs.push(...inlineToRuns(bt.tokens));
    else if (bt.text) runs.push(...expandText(decodeHtml(bt.text), {}));
  }
  footnotesConfig[idx] = {
    children: [new Paragraph({
      children: runs.length ? runs : [new TextRun({ text: footnoteDefs[id], font: FONT })],
    })],
  };
});

// Auto-extract document title from the first H1, falling back to filename.
let docTitle = path.basename(MD_PATH, path.extname(MD_PATH));
for (const t of tokens) {
  if (t.type === 'heading' && t.depth === 1) {
    docTitle = (t.text || '').replace(/[\uE000-\uE1FF]/g, '').trim() || docTitle;
    break;
  }
}

// ---------- document ----------
const doc = new Document({
  creator: 'md-to-docx',
  title: docTitle,
  styles: {
    default: {
      document: {
        run: { font: FONT, size: 22 },
        paragraph: { spacing: { line: 320 } },
      },
    },
    paragraphStyles: [
      { id: 'Heading1', name: 'Heading 1', basedOn: 'Normal', next: 'Normal', quickFormat: true,
        run: { size: 36, bold: true, font: FONT, color: TITLE_COLOR },
        paragraph: { spacing: { before: 480, after: 240 }, outlineLevel: 0,
          border: { bottom: { style: BorderStyle.SINGLE, size: 12, color: TITLE_COLOR, space: 4 } } } },
      { id: 'Heading2', name: 'Heading 2', basedOn: 'Normal', next: 'Normal', quickFormat: true,
        run: { size: 30, bold: true, font: FONT, color: ACCENT_COLOR },
        paragraph: { spacing: { before: 360, after: 180 }, outlineLevel: 1,
          border: { bottom: { style: BorderStyle.SINGLE, size: 6, color: ACCENT_COLOR, space: 2 } } } },
      { id: 'Heading3', name: 'Heading 3', basedOn: 'Normal', next: 'Normal', quickFormat: true,
        run: { size: 26, bold: true, font: FONT, color: ACCENT_COLOR },
        paragraph: { spacing: { before: 280, after: 140 }, outlineLevel: 2 } },
      { id: 'Heading4', name: 'Heading 4', basedOn: 'Normal', next: 'Normal', quickFormat: true,
        run: { size: 23, bold: true, font: FONT, color: '404040' },
        paragraph: { spacing: { before: 220, after: 110 }, outlineLevel: 3 } },
      { id: 'Heading5', name: 'Heading 5', basedOn: 'Normal', next: 'Normal', quickFormat: true,
        run: { size: 22, bold: true, italics: true, font: FONT, color: '595959' },
        paragraph: { spacing: { before: 180, after: 90 }, outlineLevel: 4 } },
    ],
  },
  numbering: {
    config: [
      { reference: 'bullets', levels: [
        { level: 0, format: LevelFormat.BULLET, text: '•', alignment: AlignmentType.LEFT,
          style: { paragraph: { indent: { left: 540, hanging: 270 } } } },
        { level: 1, format: LevelFormat.BULLET, text: '◦', alignment: AlignmentType.LEFT,
          style: { paragraph: { indent: { left: 1080, hanging: 270 } } } },
        { level: 2, format: LevelFormat.BULLET, text: '▪', alignment: AlignmentType.LEFT,
          style: { paragraph: { indent: { left: 1620, hanging: 270 } } } },
      ] },
      { reference: 'numbers', levels: [
        { level: 0, format: LevelFormat.DECIMAL, text: '%1.', alignment: AlignmentType.LEFT,
          style: { paragraph: { indent: { left: 540, hanging: 360 } } } },
        { level: 1, format: LevelFormat.LOWER_LETTER, text: '%2.', alignment: AlignmentType.LEFT,
          style: { paragraph: { indent: { left: 1080, hanging: 360 } } } },
      ] },
    ],
  },
  footnotes: footnotesConfig,
  sections: [{
    properties: {
      page: {
        size: { width: pageSize.w, height: pageSize.h },
        margin: { top: MARGIN_DXA, right: MARGIN_DXA, bottom: MARGIN_DXA, left: MARGIN_DXA },
      },
    },
    children: bodyChildren,
  }],
});

Packer.toBuffer(doc).then(buf => {
  fs.writeFileSync(OUT_PATH, buf);
  console.log(`Wrote ${OUT_PATH} (${buf.length} bytes)`);
}).catch(e => { console.error(e); process.exit(1); });
