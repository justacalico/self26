#!/usr/bin/env bash
set -euo pipefail

VERSION="$1"

if [[ -z "$VERSION" ]]; then
  echo "Error: version is required" >&2
  exit 1
fi

# Update Cargo.toml package version
awk -v ver="$VERSION" '
/^\[package\]/ { in_package = 1 }
in_package && /^version = / {
  print "version = \"" ver "\""
  in_package = 0
  next
}
{ print }
' Cargo.toml > Cargo.toml.tmp && mv Cargo.toml.tmp Cargo.toml

# Update pubspec.yaml version and increment build number
if [[ -f pubspec.yaml ]]; then
  build=1
  if grep -q '^version: .*+' pubspec.yaml; then
    build=$(grep '^version: ' pubspec.yaml | sed 's/.*+//')
    build=$((build + 1))
  fi
  sed -i "s/^version: .*/version: ${VERSION}+${build}/" pubspec.yaml
fi

# Update Cargo.lock package version
if [[ -f Cargo.lock ]]; then
  awk -v ver="$VERSION" '
  /^\[\[package\]\]/ { in_pkg = 0 }
  /^name = "self26"/ { in_pkg = 1 }
  in_pkg && /^version = / {
    print "version = \"" ver "\""
    in_pkg = 0
    next
  }
  { print }
  ' Cargo.lock > Cargo.lock.tmp && mv Cargo.lock.tmp Cargo.lock
fi

# Verify
if ! grep -q "^version = \"$VERSION\"" Cargo.toml; then
  echo "Error: Cargo.toml version not updated" >&2
  exit 1
fi
if ! grep -q "^version: ${VERSION}+" pubspec.yaml; then
  echo "Error: pubspec.yaml version not updated" >&2
  exit 1
fi
