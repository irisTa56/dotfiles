/**
 * mathml-to-docx.js — Convert KaTeX MathML output to docx Math components.
 *
 * Ported from markdown-docx (MIT, github.com/vace/markdown-docx)
 * src/extensions/mathml-to-docx.ts
 *
 * Requires: docx, fast-xml-parser (resolved from skill's node_modules)
 */
'use strict';

const path = require('path');
const SKILL_ROOT = path.resolve(__dirname, '..');

function skillRequire(id) {
  return require(require.resolve(id, { paths: [SKILL_ROOT] }));
}

const {
  MathRun, MathFraction, MathRadical, MathSuperScript, MathSubScript,
  MathSubSuperScript, MathSum, MathIntegral, XmlComponent,
} = skillRequire('docx');
const { XMLParser } = skillRequire('fast-xml-parser');

// ---------- OMML Matrix helpers (not in docx-js) ----------

class MathMatrixElement extends XmlComponent {
  constructor(children) {
    super('m:e');
    for (const child of children) this.root.push(child);
  }
}

class MathMatrixRow extends XmlComponent {
  constructor(cells) {
    super('m:mr');
    for (const cell of cells) this.root.push(new MathMatrixElement(cell));
  }
}

class MathMatrix extends XmlComponent {
  constructor(rows) {
    super('m:m');
    for (const row of rows) this.root.push(new MathMatrixRow(row));
  }
}

// ---------- XML helpers ----------

function tagName(node) {
  const keys = Object.keys(node).filter(k => k !== 'text' && k !== ':@');
  return keys[0] || null;
}

function childrenOf(node) {
  const tag = tagName(node);
  if (!tag) return [];
  const val = node[tag];
  return Array.isArray(val) ? val : (val ? [val] : []);
}

function textFrom(nodes) {
  const texts = nodes.map(n => (n.text ?? '').toString()).join('');
  return texts ? [new MathRun(texts)] : [];
}

function directText(nodes) {
  return nodes.map(n => (n.text ?? '').toString()).join('');
}

function findFirst(nodes, name) {
  for (const n of nodes) {
    if (tagName(n) === name) return n;
    const inner = findFirst(childrenOf(n), name);
    if (inner) return inner;
  }
  return null;
}

function firstN(nodes, n) {
  return nodes.slice(0, n);
}

function naryAsSubSup(op, lower, upper, body) {
  return [
    new MathSubSuperScript({
      children: [new MathRun(op)],
      subScript: lower,
      superScript: upper,
    }),
    ...body,
  ];
}

// ---------- Core walkers ----------

let loCompat = false;

function walkChildren(nodes) {
  let out = [];
  for (let i = 0; i < nodes.length; i++) {
    const n = nodes[i];
    const tag = tagName(n);

    // Handle NAry operators with limits (munderover, munder, mover)
    if (tag === 'munderover' || tag === 'munder' || tag === 'mover') {
      const kids = childrenOf(n);
      const moNode = findFirst(kids, 'mo');
      const opText = moNode ? directText(childrenOf(moNode)) : '';
      const lower = tag !== 'mover' ? (kids[1] ? walkNode(kids[1]) : []) : [];
      const upper = tag !== 'munder' ? (kids[2] ? walkNode(kids[2]) : []) : [];
      const base = walkChildren(nodes.slice(i + 1));
      if (opText.includes('∑')) {
        if (loCompat) {
          out.push(...naryAsSubSup('∑', lower, upper, base));
        } else {
          out.push(new MathSum({ children: base, subScript: lower, superScript: upper }));
        }
        break;
      }
      if (opText.includes('∫')) {
        if (loCompat) {
          out.push(...naryAsSubSup('∫', lower, upper, base));
        } else {
          out.push(new MathIntegral({ children: base, subScript: lower, superScript: upper }));
        }
        break;
      }
      // Unrecognized operator — fall through
    }

    // KaTeX uses msubsup around the operator (mo)
    if (tag === 'msubsup') {
      const ks = childrenOf(n);
      const base = ks[0];
      if (tagName(base) === 'mo') {
        const op = directText(childrenOf(base));
        const lower = ks[1] ? walkNode(ks[1]) : [];
        const upper = ks[2] ? walkNode(ks[2]) : [];
        const body = walkChildren(nodes.slice(i + 1));
        if (op.includes('∑')) {
          out.push(...(loCompat
            ? naryAsSubSup('∑', lower, upper, body)
            : [new MathSum({ children: body, subScript: lower, superScript: upper })]));
          break;
        }
        if (op.includes('∫')) {
          out.push(...(loCompat
            ? naryAsSubSup('∫', lower, upper, body)
            : [new MathIntegral({ children: body, subScript: lower, superScript: upper })]));
          break;
        }
      }
    }

    out = out.concat(walkNode(n));
  }
  return out;
}

function walkNode(node) {
  const tag = tagName(node);
  if (!tag) {
    const t = (node.text ?? '').toString();
    return t ? [new MathRun(t)] : [];
  }
  const kids = childrenOf(node);

  switch (tag) {
    case 'mrow':
      return walkChildren(kids);

    case 'mi':
    case 'mn':
    case 'mo':
      return textFrom(kids);

    case 'msup': {
      const [base, sup] = firstN(kids, 2);
      return [new MathSuperScript({ children: walkNode(base), superScript: walkNode(sup) })];
    }
    case 'msub': {
      const [base, sub] = firstN(kids, 2);
      return [new MathSubScript({ children: walkNode(base), subScript: walkNode(sub) })];
    }
    case 'msubsup': {
      const [base, sub, sup] = firstN(kids, 3);
      return [new MathSubSuperScript({
        children: walkNode(base),
        subScript: walkNode(sub),
        superScript: walkNode(sup),
      })];
    }
    case 'mfrac': {
      const [num, den] = firstN(kids, 2);
      return [new MathFraction({ numerator: walkNode(num), denominator: walkNode(den) })];
    }
    case 'msqrt': {
      const [body] = firstN(kids, 1);
      return [new MathRadical({ children: walkNode(body) })];
    }
    case 'mroot': {
      const [body, degree] = firstN(kids, 2);
      return [new MathRadical({ children: walkNode(body), degree: walkNode(degree) })];
    }
    case 'mtable': {
      const rows = kids.filter(k => tagName(k) === 'mtr');
      if (loCompat) {
        // LibreOffice-friendly: bracketed [row1; row2; ...]
        const parts = [];
        parts.push(new MathRun('['));
        rows.forEach((row, ri) => {
          if (ri > 0) parts.push(new MathRun('; '));
          const cells = childrenOf(row).filter(c => tagName(c) === 'mtd');
          cells.forEach((cell, ci) => {
            if (ci > 0) parts.push(new MathRun(', '));
            parts.push(...walkChildren(childrenOf(cell)));
          });
        });
        parts.push(new MathRun(']'));
        return parts;
      }
      // True OMML matrix
      const rowsCells = rows.map(row => {
        const cells = childrenOf(row).filter(c => tagName(c) === 'mtd');
        return cells.map(cell => walkChildren(childrenOf(cell)));
      });
      return [new MathMatrix(rowsCells)];
    }
    case 'munderover':
    case 'munder':
    case 'mover': {
      const m = childrenOf(node);
      const op = textFrom(childrenOf(findFirst(m, 'mo') || {}));
      const low = tag !== 'mover' ? (m[1] ? walkNode(m[1]) : []) : [];
      const up = tag !== 'munder' ? (m[2] ? walkNode(m[2]) : []) : [];
      return op.concat(low).concat(up);
    }
    case 'mspace':
      return [new MathRun(' ')];
    case 'mtext':
      return textFrom(kids);
    default:
      return walkChildren(kids);
  }
}

// ---------- Public API ----------

/**
 * Convert a KaTeX MathML string to an array of docx MathComponent objects.
 * @param {string} mathml  The MathML string from katex.renderToString()
 * @param {object} [opts]
 * @param {boolean} [opts.libreOfficeCompat=false]  Prefer constructs that
 *   render reliably in LibreOffice (matrices as bracketed text, n-ary ops
 *   as sub/superscripts).
 * @returns {import('docx').MathComponent[]}
 */
function mathmlToDocxChildren(mathml, opts) {
  const parser = new XMLParser({
    ignoreAttributes: false,
    attributeNamePrefix: '',
    textNodeName: 'text',
    preserveOrder: true,
    trimValues: false,
  });
  const json = parser.parse(mathml);
  const mathNode = findFirst(json, 'math');
  loCompat = !!(opts && opts.libreOfficeCompat);

  if (!mathNode) return [];
  const semantics = findFirst(childrenOf(mathNode), 'semantics');
  const root = semantics
    ? (findFirst(childrenOf(semantics), 'mrow') || semantics)
    : (findFirst(childrenOf(mathNode), 'mrow') || mathNode);
  return walkChildren(childrenOf(root));
}

module.exports = { mathmlToDocxChildren };
