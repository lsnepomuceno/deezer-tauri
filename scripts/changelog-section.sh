#!/bin/sh
# Prints the section of CHANGELOG.md for a version (with or without the leading "v"),
# and fails when there is none. Used by .github/workflows/release.yml.
set -eu

version="${1#v}"
changelog="${2:-CHANGELOG.md}"

section=$(awk -v heading="## [$version]" '
  index($0, heading) == 1 { found = 1; next }
  found && /^## \[/ { exit }
  found && /^\[[^]]+\]: / { exit }
  found { print }
' "$changelog" | sed -e '/./,$!d' -e :a -e '/^\n*$/{$d;N;ba' -e '}')

if [ -z "$section" ]; then
  echo "CHANGELOG.md has no section for $version" >&2
  exit 1
fi
printf '%s\n' "$section"
