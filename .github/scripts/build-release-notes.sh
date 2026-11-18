#!/usr/bin/env bash
set -euo pipefail

version="${TAG#v}"

extract_section() {
  awk -v version="$version" '
    BEGIN { expected = "## [" version "]" }
    /^## \[/ {
      if (found) exit
      if (substr($0, 1, length(expected)) == expected) found = 1
    }
    found { print }
    END {
      if (!found) {
        printf "::error file=%s::No changelog section found for version %s\n", FILENAME, version > "/dev/stderr"
        exit 1
      }
    }
  ' "$1"
}

en_notes="$(extract_section CHANGELOG.md)"
zh_notes="$(extract_section CHANGELOG.zh-CN.md)"
printf '%s\n\n---\n\n%s\n' "$en_notes" "$zh_notes"
