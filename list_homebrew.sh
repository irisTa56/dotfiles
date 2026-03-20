#!/bin/bash
set -euxo pipefail

brew list --cask -1 > homebrew_casks.txt
brew leaves > homebrew_leaves.txt
