# Changelog

## [0.9.0](https://github.com/irisTa56/dotfiles/compare/dotfiles-tasks-v0.8.1...dotfiles-tasks-v0.9.0) (2026-11-21)


### Features

* **local:** parse dotfiles-local arguments with clap and rename subcommands by the naming rule ([#503](https://github.com/irisTa56/dotfiles/issues/503)) ([30e1ab5](https://github.com/irisTa56/dotfiles/commit/30e1ab54fb96550e846a6d910526e07055763958))
* **tasks:** parse dotfiles-tasks arguments with clap ([#498](https://github.com/irisTa56/dotfiles/issues/498)) ([be368d7](https://github.com/irisTa56/dotfiles/commit/be368d7716d9514f309aa16bf7af5d25c47dc591))

## [0.8.1](https://github.com/irisTa56/dotfiles/compare/dotfiles-tasks-v0.8.0...dotfiles-tasks-v0.8.1) (2026-11-21)


### Bug Fixes

* **tasks:** install the dotfiles-tasks release without running the consumer's hooks ([#493](https://github.com/irisTa56/dotfiles/issues/493)) ([5a54522](https://github.com/irisTa56/dotfiles/commit/5a54522466cc0c6c2cf39e3f1a458273fadd7b47))

## [0.8.0](https://github.com/irisTa56/dotfiles/compare/dotfiles-tasks-v0.7.0...dotfiles-tasks-v0.8.0) (2026-11-21)


### Features

* **local:** add block-bsd-sed-inplace as a dotfiles-local subcommand ([#467](https://github.com/irisTa56/dotfiles/issues/467)) ([49d22d1](https://github.com/irisTa56/dotfiles/commit/49d22d16bad2bab0270769a25becd1596a183fa5))
* **local:** add find-kboat-repo's commands as dotfiles-local subcommands ([#485](https://github.com/irisTa56/dotfiles/issues/485)) ([fec764f](https://github.com/irisTa56/dotfiles/commit/fec764f9efc7fcbdfe0419a39b83a7014040df30))

## [0.7.0](https://github.com/irisTa56/dotfiles/compare/dotfiles-tasks-v0.6.0...dotfiles-tasks-v0.7.0) (2026-11-20)


### Features

* **local:** run skills:update through dotfiles-local ([#459](https://github.com/irisTa56/dotfiles/issues/459)) ([0d95026](https://github.com/irisTa56/dotfiles/commit/0d95026b671891bc3cfb6806c4fb5e314531530c))

## [0.6.0](https://github.com/irisTa56/dotfiles/compare/dotfiles-tasks-v0.5.0...dotfiles-tasks-v0.6.0) (2026-11-19)


### Features

* **tasks:** open the pin move's pull request from upstream:pin-check --bump ([#453](https://github.com/irisTa56/dotfiles/issues/453)) ([3d8de01](https://github.com/irisTa56/dotfiles/commit/3d8de014d214fbcf9be4dbe5bf913308e77cb977))

## [0.5.0](https://github.com/irisTa56/dotfiles/compare/dotfiles-tasks-v0.4.0...dotfiles-tasks-v0.5.0) (2026-11-19)


### Features

* **tasks:** add secrets-push-scan to dotfiles-tasks ([#449](https://github.com/irisTa56/dotfiles/issues/449)) ([47cac9f](https://github.com/irisTa56/dotfiles/commit/47cac9f20d65a0068f27e9504230fc463d3654d5))

## [0.4.0](https://github.com/irisTa56/dotfiles/compare/dotfiles-tasks-v0.3.0...dotfiles-tasks-v0.4.0) (2026-11-19)


### Features

* **tasks:** name each subcommand and its modules after its mise task ([#445](https://github.com/irisTa56/dotfiles/issues/445)) ([dad4c32](https://github.com/irisTa56/dotfiles/commit/dad4c32f3f0630cb40d65f2b59005fc5ed0eb052))

## [0.3.0](https://github.com/irisTa56/dotfiles/compare/dotfiles-tasks-v0.2.1...dotfiles-tasks-v0.3.0) (2026-11-19)


### Features

* **tasks:** add a link-check-staged subcommand that checks the links a commit stages ([#442](https://github.com/irisTa56/dotfiles/issues/442)) ([52834f1](https://github.com/irisTa56/dotfiles/commit/52834f1c3aa97d042f6b158774e1970132d65f0e))

## [0.2.1](https://github.com/irisTa56/dotfiles/compare/dotfiles-tasks-v0.2.0...dotfiles-tasks-v0.2.1) (2026-11-19)


### Bug Fixes

* **tasks:** survive a closed stdout or stderr in pin-check ([#438](https://github.com/irisTa56/dotfiles/issues/438)) ([ae9201b](https://github.com/irisTa56/dotfiles/commit/ae9201be13189bc860e09a789bce67b6033b2e4d))
* **tasks:** write every task's output through one closed-pipe-safe stream ([#441](https://github.com/irisTa56/dotfiles/issues/441)) ([18f67b4](https://github.com/irisTa56/dotfiles/commit/18f67b44f17b884ed2fc6fdd9fb4a7d7653a9e04))

## [0.2.0](https://github.com/irisTa56/dotfiles/compare/dotfiles-tasks-v0.1.0...dotfiles-tasks-v0.2.0) (2026-11-19)


### Features

* **tasks:** add a size-check subcommand that measures tracked skills ([#436](https://github.com/irisTa56/dotfiles/issues/436)) ([5d804b3](https://github.com/irisTa56/dotfiles/commit/5d804b3e626066ebb8a48d333c0abd22387c1ef5))

## 0.1.0 (2026-11-19)


### Features

* **tasks:** check pinned upstream paths with upstream:pin-check, run from a released Rust binary ([#427](https://github.com/irisTa56/dotfiles/issues/427)) ([e86f35d](https://github.com/irisTa56/dotfiles/commit/e86f35dabf9aca7cf4acf15f1484ca62145e57fb))
