---
name: md-to-docx
description: Convert an existing Markdown file into a polished, ready-to-share .docx using docx-js. Use when the user wants to turn a markdown document (with tables, images, code blocks, footnotes, TeX math, links) into a Word document that looks good without manual cleanup. Triggers include "md を docx に変換", "markdown to docx", "Word 化", "docx 化", or any request that takes a `.md` file as input and expects a styled `.docx` as output. Distinct from the general `docx` skill, which focuses on creating new docx files from scratch or editing existing ones — prefer this skill whenever the source is markdown.
---

# md-to-docx

Convert a Markdown file to a styled `.docx` via a Node.js script that uses [docx-js](https://github.com/dolanmiu/docx) and [marked](https://github.com/markedjs/marked).

The defaults reproduce a polished bilingual (Japanese / English) report style — content-aware table column widths, Yu Gothic typography, blue heading hierarchy with rules, monospaced code blocks with light shading, native Word footnotes, and TeX → Unicode math fallback.

## When to use

- The user provides an existing `.md` file and asks for a `.docx`
- The result should be readable as-is, without opening Word to fix layout
- The document contains any of: pipe tables, images, code blocks, footnotes, inline / display math, hyperlinks, CJK text
- For **creating** a new Word document from scratch, prefer the general `docx` skill instead

## Prerequisites

Dependencies are managed via `package.json` in the skill directory. Install once:

```bash
cd ~/.claude/skills/md-to-docx && npm ci
```

All packages (docx, marked, katex, fast-xml-parser) are pinned in `package-lock.json`.
No global installs are required.

## Usage

```bash
node ~/.claude/skills/md-to-docx/scripts/build_docx.js <input.md> <output.docx> [options]
```

After generating the file, briefly verify it (e.g., unzip and check `word/document.xml` is well-formed, count tables / drawings / footnote references) and report the result to the user with a markdown link to the output.

### Options

All options are optional. Omitting them reproduces the default style exactly.

| Flag | Default | Description |
|---|---|---|
| `--font <name>` | `Yu Gothic` | Body font (applied to ASCII and East Asian characters) |
| `--mono <name>` | `Menlo` | Monospace font for inline code and code blocks |
| `--page <a4\|letter>` | `a4` | Page size |
| `--margin <inches>` | `1` | Margin on all four sides, in inches |
| `--title-color <hex>` | `1F3864` | H1 color (without `#`) |
| `--accent <hex>` | `2E74B5` | H2/H3 color and accent rules (without `#`) |
| `--max-image-px <n>` | `580` | Max image width in display pixels (height scales proportionally) |
| `--resource-path <p1:p2>` | (md dir) | Extra colon-separated directories to search for image files |
| `--math-engine <engine>` | `auto` | Math engine: `auto` (KaTeX if available, else builtin), `katex`, or `builtin` |

### Examples

```bash
# Default style — matches the reference report exactly
node build_docx.js report.md report.docx

# US Letter, Hiragino font for macOS native look
node build_docx.js report.md report.docx --page letter --font "Hiragino Sans"

# Crimson accent (e.g., for an internal critical-issue report)
node build_docx.js report.md report.docx --title-color 800000 --accent C00000

# Image search across multiple roots
node build_docx.js report.md report.docx --resource-path "../assets:../../shared/images"
```

## Default style summary

| Element | Style |
|---|---|
| Page | A4 portrait, 1 inch margins (content width = 9026 DXA) |
| Body | Yu Gothic 11pt, line spacing 1.33 |
| H1 | 18pt, navy `#1F3864`, bold, bottom border |
| H2 | 15pt, blue `#2E74B5`, bold, thin bottom border |
| H3 | 13pt, blue `#2E74B5`, bold |
| H4 | 11.5pt, dark gray `#404040`, bold |
| H5 | 11pt, gray `#595959`, bold italic |
| Tables | Content-aware column widths (CJK chars weighted 2×), light blue header `#DEEAF6`, gray borders `#BFBFBF`, repeating header on page break |
| Code blocks | Menlo 9pt, gray background `#F5F5F5`, indented, top border |
| Inline code | Menlo, light gray shading `#F2F2F2` |
| Images | PNG / JPEG / GIF, centered, italic gray caption from alt text, max 580px wide |
| Footnotes | Native Word footnotes from `[^id]` markers |
| Math (katex) | TeX → KaTeX MathML → native Word OMML: fractions, radicals, sub/superscripts, sums, integrals, matrices |
| Math (builtin) | TeX → Unicode fallback in Word equation objects: `\frac{a}{b}` → `(a) ÷ (b)`, `\geq` → `≥`, etc. |
| Lists | `LevelFormat.BULLET` (•/◦/▪) and `DECIMAL`, hanging indent |
| Hyperlinks | Blue `#2E74B5` underlined |

## Limitations

- **Headings**: H1–H5 supported. H6 is rendered with the H5 style.
- **Images**: PNG, JPEG, and GIF only. SVG and other formats are skipped with a warning.
- **Math**: With `katex` engine, rendered as native Word equations (OMML) supporting fractions, radicals, sub/superscripts, sums, integrals, and matrices. With `builtin` engine, rendered as Unicode text inside Word equation objects.
- **Code blocks**: fenced (```` ``` ````) and inline (`` ` ``) only. Indented (4-space) code blocks are not protected from math/footnote pre-processing.
- **Footnote definitions**: single-line only. GFM continuation lines (blank lines or 4-space-indented continuations forming multi-paragraph footnotes) are not captured — only the first line of each `[^id]: ...` definition becomes the footnote body.
- **Hard-coded colors**: caption gray, H4 dark gray, H5 gray, code-block borders, and table borders are not affected by `--accent` / `--title-color`.

## Features

- **Content-aware table columns**: each column's width is computed from the maximum display width of its header + body cells (CJK characters count as 2, ASCII as 1, capped at 40 to allow wrapping), then proportionally distributed across the content area. Eliminates the squashed-column problem of pandoc.
- **Japanese typography**: the body font is applied with the `eastAsia` hint, ensuring proper rendering of CJK text without falling back to a default font.
- **Footnote handling**: `[^id]: ...` definitions are stripped from the body and inserted into the document's footnotes config; `[^id]` markers in the body become `FootnoteReferenceRun` instances.
- **Math rendering**: Two-tier engine: (1) KaTeX → MathML → native Word OMML equations with proper fractions (`m:f`), radicals (`m:rad`), sub/superscripts, sums, integrals, and matrices; (2) builtin TeX → Unicode fallback wrapped in Word equation objects (`m:oMath` + `m:r`). Engine is selected automatically (`auto`) or explicitly via `--math-engine`.
- **Image resolution**: relative image paths are resolved against the markdown file's directory, plus any extra `--resource-path` entries. PNG dimensions are read from the file header to compute the correct aspect ratio.
- **Inline formatting**: bold, italic, strikethrough, inline code, links — all rendered with the appropriate styling.

## Troubleshooting

- **`Cannot find module 'docx'`** — run `cd ~/.claude/skills/md-to-docx && npm ci`.
- **Math equations are Unicode text instead of native Word equations** — `katex` and `fast-xml-parser` are included in `package.json`; ensure `npm ci` completed without errors.
- **CJK characters appear as boxes** — the chosen font is missing on the system. Try `--font "Hiragino Sans"` (macOS), `--font "MS Gothic"` (Windows), or `--font "Noto Sans CJK JP"` (Linux).
- **A table column is still too narrow** — check whether the header text is much shorter than the body cells. The width heuristic uses `max(header, body)` but caps at 40 chars per cell to allow wrapping; if a body cell is much wider than the header, the column gets proportionally more space already.
- **Image not found** — pass `--resource-path` with the directory containing the image, or fix the relative path in the markdown.
- **Math is not rendered as a Word equation** — run `cd ~/.claude/skills/md-to-docx && npm ci` and ensure `--math-engine` is `auto` (default) or `katex`. If KaTeX fails on a specific expression, the builtin Unicode fallback is used for that expression.
- **Output looks different from the reference report** — verify that no flags were passed; defaults reproduce the reference exactly. If flags were passed, only the flagged values change; everything else stays at the defaults.

## Verification checklist

After generating, run a quick sanity check:

```bash
unzip -p output.docx word/document.xml | python3 -c "
import sys, re, xml.etree.ElementTree as ET
xml = sys.stdin.read()
ET.fromstring(xml)  # raises if malformed
print('XML well-formed')
print('tables:', len(re.findall(r'<w:tblGrid>', xml)))
print('drawings:', xml.count('<w:drawing>'))
print('footnoteRefs:', xml.count('<w:footnoteReference'))
"
```

Then report the output path to the user as a markdown link, e.g. `[output.docx](path/to/output.docx)`.
