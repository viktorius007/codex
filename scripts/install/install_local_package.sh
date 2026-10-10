#!/bin/sh
# Installs one built local Codex package and points the PATH links at it.
#
# Usage: install_local_package.sh PACKAGE_DIR [LIB_ROOT [BIN_DIR]]
#   PACKAGE_DIR  package directory written by scripts/build_codex_package.py
#   LIB_ROOT     default: $HOME/.local/lib/codex-local
#   BIN_DIR      default: $HOME/.local/bin
#
# The package is copied to LIB_ROOT/<version>-<sha12 of both binaries>. LIB_ROOT/current
# and the BIN_DIR links resolve through LIB_ROOT/current, so they never point
# at a package directory or a temporary directory. A failed verification after
# the switch restores the previous current link.
set -eu

if [ "$#" -lt 1 ] || [ "$#" -gt 3 ]; then
  echo "usage: $0 PACKAGE_DIR [LIB_ROOT [BIN_DIR]]" >&2
  exit 2
fi
package_dir=$1
lib_root=${2:-$HOME/.local/lib/codex-local}
bin_dir=${3:-$HOME/.local/bin}
for path in "$package_dir" "$lib_root" "$bin_dir"; do
  case $path in
    /*) ;;
    *) echo "paths must be absolute: $path" >&2; exit 2 ;;
  esac
done

for required in codex-package.json bin/codex bin/codex-code-mode-host; do
  if [ ! -f "$package_dir/$required" ]; then
    echo "package is missing $required: $package_dir" >&2
    exit 1
  fi
done

version=$(python3 -I -c 'import json, sys; print(json.load(open(sys.argv[1]))["version"])' "$package_dir/codex-package.json")
package_sha=$(cat "$package_dir/bin/codex" "$package_dir/bin/codex-code-mode-host" | shasum -a 256 | cut -c1-12)
expected_version="codex-cli $version"
actual_version=$("$package_dir/bin/codex" --version)
if [ "$actual_version" != "$expected_version" ]; then
  echo "package reports '$actual_version', expected '$expected_version'" >&2
  exit 1
fi
"$package_dir/bin/codex-code-mode-host" --help >/dev/null

for name in codex codex-code-mode-host; do
  link="$bin_dir/$name"
  if [ -e "$link" ] && [ ! -L "$link" ]; then
    echo "refusing to replace a non-link file: $link" >&2
    exit 1
  fi
done
if [ -e "$lib_root/current" ] && [ ! -L "$lib_root/current" ]; then
  echo "refusing to replace a non-link file: $lib_root/current" >&2
  exit 1
fi

mkdir -p "$lib_root" "$bin_dir"
release="$lib_root/$version-$package_sha"
if [ -e "$release" ]; then
  for name in codex codex-code-mode-host; do
    if ! cmp -s "$release/bin/$name" "$package_dir/bin/$name"; then
      echo "existing release differs from the package: $release" >&2
      exit 1
    fi
  done
else
  staging="$lib_root/.staging-$$"
  cp -R "$package_dir" "$staging"
  mv "$staging" "$release"
fi

previous=none
if [ -L "$lib_root/current" ]; then
  previous=$(readlink "$lib_root/current")
fi

# Replaces a symlink in one rename, so readers see the old or the new target.
point_link() {
  tmp="$1.new-$$"
  ln -s "$2" "$tmp"
  mv -fh "$tmp" "$1"
}

rollback() {
  if [ "$previous" = none ]; then
    unlink "$bin_dir/codex"
    unlink "$bin_dir/codex-code-mode-host"
    unlink "$lib_root/current"
  else
    point_link "$lib_root/current" "$previous"
  fi
}

point_link "$lib_root/current" "$release"
for name in codex codex-code-mode-host; do
  point_link "$bin_dir/$name" "$lib_root/current/bin/$name"
done

if [ "$("$bin_dir/codex" --version)" != "$expected_version" ] \
  || ! cmp -s "$bin_dir/codex" "$package_dir/bin/codex" \
  || ! "$bin_dir/codex-code-mode-host" --help >/dev/null; then
  echo "installed links failed verification; restoring previous current link" >&2
  rollback
  exit 1
fi

printf '%s installed %s previous=%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$release" "$previous" >>"$lib_root/install-log.txt"
echo "installed $expected_version at $release"
echo "links: $bin_dir/codex and $bin_dir/codex-code-mode-host -> $lib_root/current/bin"
