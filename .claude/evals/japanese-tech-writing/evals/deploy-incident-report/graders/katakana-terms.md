---
type: llm
---

The response is a Japanese incident report summarizing an English log about a failed deployment.
The log uses merge, build, cache, deploy, roll back, release and artifact.
Japanese developers call these マージ, ビルド, キャッシュ, デプロイ, ロールバック, リリース and アーティファクト (or 成果物), and that is what a Japanese reader expects to see.

PASS if, everywhere the report refers to one of these concepts, it uses the katakana term, 成果物 for the artifact, バージョン for the release, or a plain wording a Japanese developer would read the same way without translating back, such as 元に戻す or 切り戻す for rolling back or 本番環境に反映する for deploying.

FAIL if the report renders any of them with a kanji or native Japanese word that a Japanese developer would not use for it, such as 統合 or 併合 for merge, 構築 for build, 一時保存 or 一時記憶 for cache, 配備 or 展開 for deploy, or 版 for release.
