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

zaphod_require_zellij_0443() {
    local version
    if ! command -v zellij >/dev/null 2>&1; then
        echo "zellij 0.44.3 is required" >&2
        return 1
    fi
    version="$(zellij --version)" || return 1
    if [ "$version" != "zellij 0.44.3" ]; then
        echo "zellij 0.44.3 is required; found $version" >&2
        return 1
    fi
}

zaphod_message_plugin_entries() {
    local config_file="$1"
    awk '
        function emit_entry() {
            print plugin_url "\t" (has_rail ? "1" : "0")
            in_plugin = 0
            plugin_url = ""
            has_rail = 0
            depth = 0
        }
        {
            line = $0
            opens = gsub(/\{/, "{", line)
            closes = gsub(/\}/, "}", line)
            if (!in_plugin && match(line, /MessagePlugin[[:space:]]+"[^"]+"/)) {
                plugin_url = substr(line, RSTART, RLENGTH)
                sub(/^MessagePlugin[[:space:]]+"/, "", plugin_url)
                sub(/"$/, "", plugin_url)
                if (plugin_url ~ /(^|\/)zellij-sidebar\.wasm($|[?#])/) {
                    in_plugin = 1
                    depth = opens - closes
                    has_rail = line ~ /rail[[:space:]]+"1"/
                    if (depth <= 0) {
                        emit_entry()
                    }
                }
            } else if (in_plugin) {
                depth += opens - closes
                if (line ~ /rail[[:space:]]+"1"/) {
                    has_rail = 1
                }
                if (depth <= 0) {
                    emit_entry()
                }
            }
        }
        END {
            if (in_plugin) {
                emit_entry()
            }
        }
    ' "$config_file"
}

zaphod_validate_message_plugin_identity() {
    local config_file="$1"
    local expected_url="$2"
    local entries url has_rail errors
    if [ ! -f "$config_file" ]; then
        echo "Zaphod config not found: $config_file" >&2
        return 1
    fi
    entries="$(zaphod_message_plugin_entries "$config_file")" || return 1
    if [ -z "$entries" ]; then
        echo "no Zaphod MessagePlugin keybinds found in $config_file" >&2
        return 1
    fi

    errors=0
    while IFS="$(printf '\t')" read -r url has_rail; do
        if [ "$url" != "$expected_url" ]; then
            echo "Zaphod MessagePlugin URL mismatch: $url" >&2
            echo "expected: $expected_url" >&2
            errors=1
        fi
        if [ "$has_rail" != "1" ]; then
            echo "Zaphod MessagePlugin missing rail \"1\": $url" >&2
            errors=1
        fi
    done <<EOF
$entries
EOF
    [ "$errors" -eq 0 ]
}
