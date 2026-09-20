#!/usr/bin/env bash
#MISE description="Scan staged changes for secrets"
#MISE tools={ gitleaks = "latest" }
set -euo pipefail

# The repository, not the working directory. A task runs at the mise project root, which is
# the directory of the nearest mise config and need not be the git root: where that config
# sits below it, gitleaks scopes the staged scan to that subtree and reports nothing for a
# secret staged outside it. git's own message prints when there is no repository to name.
root=$(git rev-parse --show-toplevel)

gitleaks git --staged -v "$root"
