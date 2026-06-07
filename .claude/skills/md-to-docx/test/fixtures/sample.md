# Smoke Test Document

A minimal Markdown document that exercises the main features of the
`build_docx.js` converter so dependency upgrades break the build loudly.

## Text features

This paragraph has **bold**, *italic*, `inline code`, and a
[link to example.com](https://example.com).

- bullet one
- bullet two
  - nested bullet

1. ordered one
2. ordered two

> A block quote.

## Table

| Name | Value |
| ---- | ----- |
| foo  | 1     |
| bar  | 2     |

## Code block

```js
const x = 1 + 2;
console.log(x);
```

## Math

Inline math $E = mc^2$ and a display block:

$$
\int_0^1 x^2 \, dx = \frac{1}{3}
$$

A footnote reference.[^1]

[^1]: The footnote body.

<!--
  Smoke-test guard: do NOT add \div (or anything else that renders to "÷") to
  this fixture. test/smoke-test.mjs treats a literal "÷" as proof that the
  builtin math fallback ran (a KaTeX/fast-xml-parser regression). KaTeX renders
  \div to "÷" as well, so adding it would make that check false-positive.
-->
