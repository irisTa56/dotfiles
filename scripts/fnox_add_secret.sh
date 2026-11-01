#!/usr/bin/env bash
set -euo pipefail

# Usage: fnox-add-secret <NAME>
# Stores an API key in the keychain for this repository, through fnox; NAME is the
# environment variable the key is passed as.
# The README's Secrets section is the convention this command carries out.

if [[ $# -ne 1 ]]; then
  echo "Usage: fnox-add-secret <NAME>" >&2
  exit 1
fi

# Appending to a file whose last line lacks a newline would glue the two.
end_line() { if [[ -s $1 && -n "$(tail -c1 "$1")" ]]; then echo >>"$1"; fi; }

# The main checkout, not a worktree: fnox.local.toml is git-ignored,
# so a worktree has none, and fnox finds the main checkout's from one under it.
root="$(git worktree list --porcelain | sed -n '1s/^worktree //p')"
cd "$root"
touch fnox.local.toml

# The provider is named after the repository: fnox lists every provider it loads,
# from parent directories and the global config too, and one of theirs
# would store the key under their keychain service rather than this one's.
# fnox does the parsing, so a declaration in any TOML form counts,
# and a file it cannot parse stops the command.
provider="$(basename "$root")"
providers="$(fnox provider list -c fnox.local.toml)"
if ! grep -qxF "$provider" <<<"$providers"; then
  # Some TOML shapes cannot take an appended table (an inline `providers = {…}`),
  # so the declaration goes into a copy that replaces the file only once fnox parses it.
  trap 'rm -f fnox.local.toml.new' EXIT
  cp fnox.local.toml fnox.local.toml.new
  end_line fnox.local.toml.new
  printf '[providers."%s"]\ntype = "keychain"\nservice = "%s"\n\n' "$provider" "$provider" >>fnox.local.toml.new
  if ! fnox provider list -c fnox.local.toml.new >/dev/null; then
    echo "fnox.local.toml cannot take the declaration as appended; left unchanged, so add a keychain provider named $provider by hand" >&2
    exit 1
  fi
  mv fnox.local.toml.new fnox.local.toml
fi

# `fnox set` writes to fnox.toml when the directory has one, hence `-c`.
fnox set -c fnox.local.toml "$1" --provider "$provider"

note="$(
  cat <<'EOF'
- Secrets here come from the macOS keychain through fnox (`fnox.local.toml`), not `.env`: run a command that needs one as `fnox exec -- <command>`.
EOF
)"
# An existing file has a structure of its own, and where the line belongs in it
# is a call this command cannot make, so there the line is left to the user.
if [[ ! -e CLAUDE.local.md ]]; then
  printf '%s\n' "$note" >CLAUDE.local.md
elif ! grep -qF 'fnox exec' CLAUDE.local.md; then
  printf '%s\n%s\n' "Add this line where it fits in $root/CLAUDE.local.md:" "$note" >&2
fi
