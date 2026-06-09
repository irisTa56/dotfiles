---
description: Conventions for Japanese text in markdown documents (notes, specs, READMEs, etc.), including technical writing. Language-agnostic prose conventions live in honest-writing.md.
paths:
  - "**/*.md"
---

# Japanese Writing

Follow these rules when writing Japanese text, including technical documents.

## Prefer Japanese over English when an equivalent exists

- In Japanese text, use a well-established Japanese translation (including katakana loanwords) instead of leaving the original English term.
  - Kanji translations (English shown here for cross-reference only; see §3 for when to actually annotate): 活性値 (activation), 順伝播 / 逆伝播 (forward / backward), 再計算 (recomputation), 連鎖律 (chain rule), 勾配 (gradient).
  - Katakana forms — use the katakana form when the term is used as a general concept; the English origin is self-evident: パラメータ, ステージ, マイクロバッチ, パイプライン, バッチ, デフォルト.
    - Exception: keep the original English term (not the katakana form) when the term names a concrete entity (e.g., a specific API parameter, config key, or identifier) or when cross-referencing an English source where the exact wording matters.
- Leave the following in their original form:
  - Proper nouns (e.g., PipeDream-Flush, GPipe).
  - Established acronyms (e.g., 1F1B, FLOP, LLM).
  - Direct excerpts or quotations from English-language source material.

## Keep each sentence in a single language

- Do not mix Japanese and English within a single sentence. Write each sentence fully in Japanese or fully in English.
  - NG example: 「このループは data is all starting from process 0」 — splicing a Japanese clause and an English clause within one sentence is awkward and hard to read. (Code identifiers in backticks like `DataLoaderDispatcher` are fine as proper nouns and do not count as mixing.)
- Avoid the "English verb + する" pattern (e.g., `iterate する`, `sharding する`, `gather する`). Use a Japanese verb or an established katakana form (反復する, シャーディングする, 集約する) instead.
- When preserving an English excerpt alongside Japanese commentary, place them in separate bullets rather than splicing them into one sentence. A Japanese parent bullet with an English child bullet carrying the raw excerpt is idiomatic.

## Annotate important terms with their English counterpart

- On first mention of a technically important term, write it as `和訳 (English)` so the reader can cross-reference with English sources. This parenthetical annotation is a permitted exception to §2 (no intra-sentence mixing).
  - Example: `再計算 (activation recomputation)`, `活性値 (activation)`.
- Annotate only terms central to the topic. Routine translated words (バッチ, デフォルト など) do not need an English gloss.

## Use punctuation that matches the enclosed content

- Colons: always use the half-width `:`, never the full-width `：`.
- Parentheses: match the language of the enclosed content.
  - Use full-width `（）` when the content is Japanese (even if it contains some English terms).
    - Example: `詳細は別の節（pipeline parallelism の章）を参照。`
  - Use half-width `()` when the content is purely half-width (English, code, math).
    - Example: `活性値 (activation)`, `(Narayanan et al. 2021)`.

## Space between English words and surrounding Japanese

- For a single English word adjacent to Japanese — including a compound of one English word plus Japanese — do not put a space on either side of the English word. This applies to both the boundary with preceding Japanese and the one with following Japanese.
  - OK: `Rust製`, `gRPC経由のリクエスト`, `これはElixirで書く`.
  - NG: `Rust 製`, `gRPC 経由のリクエスト`, `これは Elixirで書く`.
- Otherwise, separate the English run from the surrounding Japanese with a half-width space.
  - OK: `machine learning を学ぶ`, `REST API の設計`.
- Avoid compounds that splice consecutive English words directly onto Japanese (e.g., `REST APIサーバー`).
  - Take one of: break the English run with a Japanese particle so each English word stands alone (`RESTのAPIサーバー`), or make the whole phrase English (`REST API server`).
- This rule governs bare English words in running text. Spacing around markdown syntax (code spans, emphasis, links) follows the separate rule below.

## Put spaces around inline markdown syntax in Japanese text

- When inline markdown syntax (emphasis `**bold**` / `*italic*`, links `[text](url)`, code spans `code`, etc.) appears mid-sentence in Japanese prose, insert a half-width space before and after.
  - NG: `他方の**絶対的な**節約量`, `詳細は[§3](#annotate)を参照`.
  - OK: `他方の **絶対的な** 節約量`, `詳細は [§3](#annotate) を参照`.
- Exception: do **not** insert a space between a full-width symbol (`・`、`（`、`「` など) and the following inline markdown syntax. Full-width symbols already carry visual spacing; adding a half-width space after them creates an excessive gap.
  - NG: `あああ・ aaa`.
  - OK: `あああ・aaa`.
