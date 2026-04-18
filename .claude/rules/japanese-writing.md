---
description: Conventions for Japanese prose in markdown documents (notes, specs, READMEs, etc.). Language-agnostic prose conventions live in honest-writing.md.
paths:
  - "**/*.md"
---

# Japanese Writing

Follow these rules when writing Japanese prose.

## Prefer Japanese over English when an equivalent exists

- In Japanese text, use a well-established Japanese translation (including katakana loanwords) instead of leaving the original English term.
  - Kanji translations (English shown here for cross-reference only; see §3 for when to actually annotate): 活性値 (activation), 順伝播 / 逆伝播 (forward / backward), 再計算 (recomputation), 連鎖律 (chain rule), 勾配 (gradient).
  - Katakana forms (use as-is; the English origin is self-evident): パラメータ, ステージ, マイクロバッチ, パイプライン, バッチ, デフォルト.
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

## Put spaces around inline markdown syntax in Japanese text

- When inline markdown syntax (emphasis `**bold**` / `*italic*`, links `[text](url)`, code spans `code`, etc.) appears mid-sentence in Japanese prose, insert a half-width space before and after.
  - NG: `他方の**絶対的な**節約量`, `詳細は[§3](#annotate)を参照`.
  - OK: `他方の **絶対的な** 節約量`, `詳細は [§3](#annotate) を参照`.
