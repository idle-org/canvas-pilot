#!/bin/sh
# Publish validated badges; never sync the asset mount.
set -eu
[ "$#" -ge 2 ] || { echo "usage: $0 /absolute/publication/root coverage test version" >&2; exit 2; }
root=$1
shift
case "$root" in /*) ;; *) echo "Publication root must be absolute" >&2; exit 2;; esac
[ -d "$root" ] && [ -w "$root" ] && [ ! -L "$root" ] || { echo "Root must be an existing writable directory" >&2; exit 2; }
seen=" "
for name do
    case "$name" in coverage|test|version) ;; *) echo "Invalid badge: $name" >&2; exit 2;; esac
    case "$seen" in *" $name "*) echo "Duplicate badge: $name" >&2; exit 2;; esac
    seen="$seen$name "
    src="coverage/$name.svg"
    [ -f "$src" ] && [ -s "$src" ] && [ ! -L "$src" ] || { echo "Invalid badge source: $src" >&2; exit 2; }
done
dest=$root
for component in idle-org canvas-pilot; do
    dest="$dest/$component"
    [ ! -L "$dest" ] || { echo "Refusing directory symlink: $dest" >&2; exit 2; }
    if [ -e "$dest" ]; then
        [ -d "$dest" ] || { echo "Not a directory: $dest" >&2; exit 2; }
    else
        mkdir -m 0755 "$dest"
    fi
done
dest="$dest/main"
if [ -L "$dest" ]; then unlink "$dest"; fi
if [ -e "$dest" ]; then
    [ -d "$dest" ] || { echo "Not a directory: $dest" >&2; exit 2; }
else
    mkdir -m 0755 "$dest"
fi
temporary=
trap '[ -z "$temporary" ] || rm -f "$temporary"' EXIT HUP INT TERM
for name do
    [ ! -L "$dest/$name.svg" ] && [ ! -d "$dest/$name.svg" ] || { echo "Invalid badge destination: $name" >&2; exit 2; }
    temporary=$(mktemp "$dest/.badge.XXXXXXXX")
    cp "coverage/$name.svg" "$temporary"
    chmod 0644 "$temporary"
    mv -f "$temporary" "$dest/$name.svg"
    temporary=
done
trap - EXIT HUP INT TERM
