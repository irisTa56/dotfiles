#!/usr/bin/env node
/**
 * Smoke test for md-to-docx.
 *
 * Runs build_docx.js on a fixture Markdown file and asserts it produces a
 * valid .docx whose contents reflect the fixture. This is intentionally
 * lightweight: its job is to catch breakage from dependency upgrades
 * (Dependabot bumping docx / marked / katex / fast-xml-parser), not to assert
 * exact output.
 *
 * It does not just check "a non-empty ZIP came out" — build_docx.js silently
 * falls back to the builtin math engine when KaTeX/fast-xml-parser misbehave,
 * so a content-blind check would green-light exactly the regressions we care
 * about. Instead it unzips word/document.xml and asserts the rendered markers
 * below are present, exercising the full marked -> docx -> OMML pipeline.
 *
 * Forces --math-engine katex so the katex / fast-xml-parser path is taken.
 *
 * Usage: node test/smoke-test.mjs   (run from the skill root)
 */
import { spawnSync } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const skillRoot = dirname(here);
const builder = join(skillRoot, 'scripts', 'build_docx.js');
const fixture = join(here, 'fixtures', 'sample.md');

// Markers expected in word/document.xml, each tied to a dependency path.
// The math markers are deliberately KaTeX-specific: build_docx.js wraps BOTH
// the KaTeX path and the builtin fallback in <m:oMath>, so <m:oMath> alone
// cannot tell them apart. <m:sSup> / <m:nary> are emitted only by the
// KaTeX -> MathML -> OMML conversion, so their presence proves that path ran.
const EXPECTED_MARKERS = [
  'Smoke Test Document', // heading/body text -> marked parsing + docx serialization
  '<w:tbl>', //             GFM table        -> marked tables + docx table structure
  '<m:sSup', //             superscript (mc^2) -> KaTeX -> MathML -> OMML
  '<m:nary', //             n-ary integral (\int) -> KaTeX -> MathML -> OMML
];

// Markers that must be ABSENT. In THIS fixture the only source of a literal
// "÷" is the builtin fallback rendering \frac as "(a) ÷ (b)"; the KaTeX path
// renders \frac as <m:f>. So "÷" appearing means KaTeX/fast-xml-parser
// silently fell back — exactly the regression to catch. This relies on the
// fixture never containing \div (KaTeX would render that to "÷" too); see the
// guard note in fixtures/sample.md.
const FORBIDDEN_MARKERS = ['÷'];

function fail(msg) {
  console.error(`smoke test FAILED: ${msg}`);
  process.exit(1);
}

const tmp = mkdtempSync(join(tmpdir(), 'md-to-docx-smoke-'));
const out = join(tmp, 'out.docx');

try {
  const build = spawnSync(
    process.execPath,
    [builder, fixture, out, '--math-engine', 'katex'],
    { encoding: 'utf8', maxBuffer: 32 * 1024 * 1024 },
  );
  if (build.status !== 0) {
    fail(`builder exited with ${build.status}\n--- stdout ---\n${build.stdout}\n--- stderr ---\n${build.stderr}`);
  }
  if (!existsSync(out)) {
    fail(`output file was not created: ${out}`);
  }

  // A valid .docx is a ZIP archive; even a tiny document is well over 2 KB.
  const bytes = readFileSync(out);
  if (bytes.length < 2048) {
    fail(`output is suspiciously small (${bytes.length} bytes)`);
  }
  // ZIP local file header magic: "PK\x03\x04".
  const head = bytes.subarray(0, 4);
  if (!(head[0] === 0x50 && head[1] === 0x4b && head[2] === 0x03 && head[3] === 0x04)) {
    fail(`output is not a ZIP/.docx (magic bytes: ${[...head].map((b) => b.toString(16)).join(' ')})`);
  }

  // Inspect the rendered XML. unzip is preinstalled on the GitHub-hosted
  // ubuntu runner used by CI (and on macOS / typical local dev machines).
  const extract = spawnSync('unzip', ['-p', out, 'word/document.xml'], {
    encoding: 'utf8',
    maxBuffer: 32 * 1024 * 1024,
  });
  if (extract.status !== 0) {
    fail(`could not extract word/document.xml (unzip exited ${extract.status})\n${extract.stderr}`);
  }
  const documentXml = extract.stdout;

  const missing = EXPECTED_MARKERS.filter((m) => !documentXml.includes(m));
  if (missing.length > 0) {
    fail(`word/document.xml is missing expected content: ${missing.map((m) => JSON.stringify(m)).join(', ')}`);
  }
  const present = FORBIDDEN_MARKERS.filter((m) => documentXml.includes(m));
  if (present.length > 0) {
    fail(`word/document.xml contains content that signals a regression (builtin math fallback ran): ${present.map((m) => JSON.stringify(m)).join(', ')}`);
  }

  console.log(`smoke test OK: produced ${bytes.length}-byte .docx with expected content`);
} finally {
  rmSync(tmp, { recursive: true, force: true });
}
