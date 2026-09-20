#!/usr/bin/env bash
#MISE description="Scan staged changes for secrets"
#MISE tools={ gitleaks = "latest" }
set -euo pipefail

gitleaks git --staged -v
