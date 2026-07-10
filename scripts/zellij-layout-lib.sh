#!/bin/bash
# ABOUTME: Shared physical-path, plugin-URL, identity, and layout-rendering helpers.
# ABOUTME: Sourced by the canonical installer and disposable worktree profile.

zaphod_physical_dir() {
    (cd "$1" && pwd -P)
}

zaphod_primary_checkout_root() {
    local checkout_root="$1"
    local common_dir
    common_dir="$(git -C "$checkout_root" rev-parse --path-format=absolute --git-common-dir)" || return 1
    common_dir="$(zaphod_physical_dir "$common_dir")" || return 1
    dirname "$common_dir"
}

zaphod_canonical_file_url() {
    local path="$1"
    local directory basename
    [ -f "$path" ] || return 1
    directory="$(zaphod_physical_dir "$(dirname "$path")")" || return 1
    basename="$(basename "$path")"
    printf 'file:%s/%s\n' "$directory" "$basename"
}

zaphod_render_layout() {
    local template="$1"
    local wasm_url="$2"
    local output="$3"
    local replacement
    replacement="${wasm_url//&/\\&}"
    replacement="${replacement//|/\\|}"
    sed "s|__ZAPHOD_WASM__|$replacement|g" "$template" > "$output"
}
