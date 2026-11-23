---
type: llm
---

The response is a Japanese report summarizing an English log about setting up a developer machine.
The log says install, reinstall, upgrade and uninstall.
Japanese developers call these インストール, 再インストール, アップグレード and アンインストール, and that is what a Japanese reader expects to see.

PASS if, everywhere the report refers to installing, reinstalling, upgrading or uninstalling software, it uses those katakana terms, or a plain verb that a Japanese developer would read the same way without translating back, such as 更新する for upgrading or 削除する for uninstalling.
Commands quoted in backticks do not count.

FAIL if the report renders any of them with a kanji or native Japanese word that a Japanese developer would not use for it, such as 導入 or 入れる for install, 入れ直す or 再導入 for reinstall, or 撤去 for uninstall.
