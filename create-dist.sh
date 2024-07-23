#!/bin/bash

set -eu

files=(
    "counts-example.svg"
    "example-word.svg"
    "favicon.ico"
    "index.html"
    "puzzles.txt"
)

included_files=(
    "wordroute.js"
    "wordroute.css"
)

pkg_files=(
    "wordroute_bg.wasm"
    "wordroute.js"
)

for x in "${files[@]}" "${included_files[@]}"; do
    dn="dist/$(dirname "$x")"
    bn="$(basename "$x")"
    mkdir -p "$dn"
    cp -v "$x" "$dn/$bn"
done

pkg_md5=$(cat "${pkg_files[@]/#/pkg\//}" | md5sum - | sed 's/ .*//')
pkg_dir="dist/pkg-$pkg_md5"

mkdir -p "$pkg_dir"

for x in "${pkg_files[@]}"; do
    cp -v "pkg/$x" "$pkg_dir/$x"
done

sed -i 's|\./pkg/wordroute\.js|./pkg-'"$pkg_md5"'/wordroute.js|' \
    dist/wordroute.js

for x in "${included_files[@]}"; do
    md5=$(md5sum "dist/$x" | sed 's/ .*//')
    new_name=$(echo "$x" | sed 's/\./'"-$md5"'./')
    mv "dist/$x" "dist/$new_name"
    re_filename=$(echo "$x" | sed 's/\./\\./g')
    sed -i s/\""$re_filename"\"/\""$new_name"\"/g dist/index.html
done
