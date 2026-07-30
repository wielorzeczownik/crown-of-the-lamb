#!/usr/bin/env bash
set -euo pipefail

current_version=$(awk -F'"' '/^version[[:space:]]*=[[:space:]]*"/ { print $2; exit }' Cargo.toml)
next_tag=$(echo "${CLIFF_BUMPED_VERSION:-}" | tr -d '[:space:]')

if [[ -z "$next_tag" ]]; then
  echo "Unable to determine next version from git-cliff"
  exit 1
fi

if [[ "$next_tag" != v* ]]; then
  next_tag="v$next_tag"
fi

next_version="${next_tag#v}"
current_tag="v$current_version"
has_current_tag="false"
if git rev-parse -q --verify "refs/tags/${current_tag}" >/dev/null; then
  has_current_tag="true"
fi

if [[ "$next_version" == "$current_version" ]]; then
  if [[ "$has_current_tag" == "false" ]]; then
    echo "No bump from git-cliff, but no current tag exists. Creating initial release for ${current_tag}."
    {
      echo "released=true"
      echo "version=$current_version"
      echo "tag=$current_tag"
      echo "commit_changes=false"
    } >>"$GITHUB_OUTPUT"
    exit 0
  fi

  echo "No user-facing commits since ${current_tag}. Nothing to release."
  {
    echo "released=false"
    echo "version=$current_version"
    echo "tag=$current_tag"
    echo "commit_changes=false"
  } >>"$GITHUB_OUTPUT"
  exit 0
fi

if [[ "$(printf '%s\n' "$current_version" "$next_version" | sort -V | tail -1)" != "$next_version" ]]; then
  echo "git-cliff returned $next_version which is lower than current $current_version. Nothing to release."
  {
    echo "released=false"
    echo "version=$current_version"
    echo "tag=$current_tag"
    echo "commit_changes=false"
  } >>"$GITHUB_OUTPUT"
  exit 0
fi

# Bump the version with a plain text edit rather than cargo set-version
tmp=$(mktemp)
awk -v v="$next_version" '
  !done && /^version[[:space:]]*=[[:space:]]*"/ { sub(/"[^"]*"/, "\"" v "\""); done=1 }
  { print }
' Cargo.toml >"$tmp" && mv "$tmp" Cargo.toml

# Keep the package entry in Cargo.lock in sync
tmp=$(mktemp)
awk -v v="$next_version" '
  /^name = "crown-of-the-lamb"$/ { inpkg=1 }
  inpkg && /^version[[:space:]]*=[[:space:]]*"/ { sub(/"[^"]*"/, "\"" v "\""); inpkg=0 }
  { print }
' Cargo.lock >"$tmp" && mv "$tmp" Cargo.lock

{
  echo "released=true"
  echo "version=$next_version"
  echo "tag=$next_tag"
  echo "commit_changes=true"
} >>"$GITHUB_OUTPUT"
