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
- Separators and range marks: when every joined token is half-width (English, code, numbers), use the half-width `,` `/` `~` `-`, not their full-width counterparts. (Lists that include a Japanese token are outside this rule.)
  - NG: `Rust、Go、Python`, `foo・bar`, `1〜10`. OK: `Rust, Go, Python`, `foo / bar`, `1~10`.
- Parentheses: match the language of the enclosed content.
  - Use full-width `（）` when the content is Japanese (even if it contains some English terms).
    - Example: `詳細は別の節（pipeline parallelism の章）を参照。`
  - Use half-width `()` when the content is purely half-width (English, code, math). The separators inside then follow the half-width rule above.
    - Example: `活性値 (activation)`, `(Narayanan et al. 2021)`.
    - NG: `（foo、bar）`. OK: `(foo, bar)`.

## Space between English words and surrounding Japanese

Default to a half-width space at every boundary between an English run and adjacent Japanese. Deviate only under the cases below, and never leave a space on just one side of an English run.

- **Lexical compound → no space.** When the English and the adjacent Japanese fuse into a single word — a noun compound such as `生成AI` / `Docker環境` / `Rust言語`, or a bound affix such as `製` / `系` / `版` — a space would break the word, so join them. Test it by reading the pair back in English: if the Japanese element becomes a content word and the whole is a self-contained noun phrase (`Docker環境` = "Docker environment", `Rust言語` = "Rust language"), it is a compound; if that element becomes a preposition or other relational word (`gRPC経由` = "via gRPC"), it is not — that is the next case.
  - OK: `生成AI`, `Docker環境`, `Rust製`, `Elixir系`.
  - NG: `生成 AI`, `Docker 環境`, `Rust 製`.
- **Not a compound → avoid the bare space; rephrase.** When the pair is not a noun compound — either because the Japanese element is relational (`gRPC 経由の…` = "via gRPC") or because the English side is a multi-word term that cannot join uniformly (`Open Source 開発`) — do not leave the English and Japanese sitting side by side with only a space. Rephrase by one of:
  - Introduce a particle so the English term becomes a grammatical argument (膠着; the space stays): `gRPC を経由したデータ移動` or `gRPC によるデータ移動`, not `gRPC 経由のデータ移動`.
  - Translate or transliterate the term into Japanese — the meaning-preserving fix: `Open Source 開発` → `オープンソース開発`.
  - Collapse a multi-word English term into an established acronym, which becomes a single joinable token: `Large Language Model 基盤` → `LLM基盤`.
  - Only if none of these is possible, join with no space as a last resort (`gRPC経由`) — the accepted floor for a single English token.
- **Particle-governed standalone English → space (膠着).** When a particle (`を`, `の`, `に`, `で`, `は`, …) binds a standalone English term — one not fused into a compound on its other side — keep the default space.
  - OK: `machine learning を学ぶ`, `REST API の設計`, `API を使う`.
- **Never a one-sided space.** Spacing must be uniform across an English run — every boundary joined, or every boundary spaced, counting the mandatory space between consecutive English words as a boundary too. A particle binds its neighbor more tightly (膠着) than a plain space does, so mixing joined and spaced boundaries tears the run apart.
  - The trailing token of a compound is locked to no-space, so a following particle joins too: NG `生成AI を使う` (`AI` joined on the left, spaced on the right); OK `生成AIを使う`.
  - A multi-word English term already contains an internal space, so uniformity forces its outer boundaries to be spaced as well — it can never fuse into a compound, which is why the bare-adjacency case forces a rephrase: NG `自主的にOpen Source開発する`; OK `自主的にオープンソース開発する`.
  - Full-width punctuation and brackets (`。`, `、`, `「」`, `（）`, …) already carry visual spacing, so they count as a spaced side. An English word with such a mark on one side and a space on the other is not a violation: OK `〜。API を使う`.
- This rule governs bare English words in running text. Spacing around markdown syntax (code spans, emphasis, links) follows the separate rule below.

## Put spaces around inline markdown syntax in Japanese text

- When inline markdown syntax (emphasis `**bold**` / `*italic*`, links `[text](url)`, code spans `code`, etc.) appears mid-sentence in Japanese prose, insert a half-width space before and after.
  - NG: `他方の**絶対的な**節約量`, `詳細は[§3](#annotate)を参照`.
  - OK: `他方の **絶対的な** 節約量`, `詳細は [§3](#annotate) を参照`.
- Exception: do **not** insert a space between a full-width symbol (`・`、`（`、`「` など) and the following inline markdown syntax. Full-width symbols already carry visual spacing; adding a half-width space after them creates an excessive gap.
  - NG: `あああ・ aaa`.
  - OK: `あああ・aaa`.
