#!/bin/bash
# ABOUTME: Shared physical-path, plugin-URL, identity, and layout-rendering helpers.
# ABOUTME: Sourced by the canonical installer and disposable worktree profile.

ZAPHOD_LIVE_SESSION_NAME=""
ZAPHOD_LIVE_DATA_ROOT=""

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

# Print the one stable tab ID added between two complete native list-tabs JSON
# inventories. Any malformed record, lost prior ID, or ambiguous addition is
# rejected so callers never infer identity from action stdout or tab position.
zaphod_new_tab_id_from_inventories() {
    local before="$1"
    local after="$2"
    jq -er -n --slurpfile before "$before" --slurpfile after "$after" '
        def stable_ids:
            if type != "array" then error("tab inventory is not an array")
            else map(
                .tab_id |
                if type == "number" and . >= 0 and floor == . then tostring
                else error("tab_id is not a nonnegative integer")
                end
            ) | unique
            end;
        ($before[0] | stable_ids) as $old |
        ($after[0] | stable_ids) as $new |
        ($old - $new) as $lost |
        ($new - $old) as $added |
        if ($lost | length) == 0 and ($added | length) == 1 then $added[0]
        else error("expected exactly one added tab and no lost tabs")
        end
    '
}

zaphod_valid_tab_inventory() {
    local inventory="$1"
    jq -e '
        type == "array" and
        all(.[]; (.tab_id | type) == "number" and .tab_id >= 0 and (.tab_id | floor) == .tab_id) and
        ((map(.tab_id) | length) == (map(.tab_id) | unique | length))
    ' "$inventory" >/dev/null
}

# Format command provenance without retaining an unbounded shell value. Reply
# bodies stay in owned temporary files; only their byte counts and first 256
# bytes, JSON-escaped by jq, cross the failure boundary.
zaphod_bounded_reply_provenance() {
    local label="$1"
    local status="$2"
    local stdout_file="$3"
    local stderr_file="$4"
    local stdout_len stderr_len stdout_prefix stderr_prefix
    stdout_len="$(wc -c < "$stdout_file" | tr -d '[:space:]')"
    stderr_len="$(wc -c < "$stderr_file" | tr -d '[:space:]')"
    stdout_prefix="$(head -c 256 "$stdout_file" | jq -Rs .)"
    stderr_prefix="$(head -c 256 "$stderr_file" | jq -Rs .)"
    printf '%s status=%s stdout_len=%s stdout_prefix=%s stderr_len=%s stderr_prefix=%s' \
        "$label" "$status" "$stdout_len" "$stdout_prefix" "$stderr_len" "$stderr_prefix"
}

zaphod_panes_prove_layout_expectation() {
    local expected_url="$1"
    local expectation="$2"
    local panes_file="$3"
    case "$expectation" in
        present)
            jq -e --arg expected_url "$expected_url" '
                type == "array" and
                ([.[] | select(
                    .is_plugin == true and .plugin_url == $expected_url and
                    .is_floating == false and .is_suppressed == false
                )] | length) == 1
            ' "$panes_file" >/dev/null
            ;;
        absent)
            jq -e '
                type == "array" and
                all(.[];
                    (.is_plugin != true) or
                    ((.plugin_url // "") | test("(^|/)zellij-sidebar\\.wasm([?#].*)?$") | not)
                )
            ' "$panes_file" >/dev/null
            ;;
        *) return 2 ;;
    esac
}

# Capture one native dump-layout record, validate its entire KDL syntax and
# Zaphod identity, then atomically publish it. A valid stale identity or a
# A status-0 empty wrong-action may retry when the authoritative pane inventory
# proves either requested state. Stale identity and complete JSON wrong-action
# retries still require the exact candidate. All other failures are final.
zaphod_capture_validated_layout() {
    local validator="$1"
    local expected_url="$2"
    local expectation="$3"
    local panes_file="$4"
    local output_file="$5"
    shift 5
    local attempt_file="$output_file.attempt"
    local stderr_file="$output_file.stderr"
    local validator_stderr="$output_file.validator.stderr"
    local empty_file="$output_file.empty"
    local attempt command_status validator_status retryable provenance

    rm -f "$output_file" "$attempt_file" "$stderr_file" "$validator_stderr" "$empty_file"
    : > "$empty_file"
    if ! zaphod_panes_prove_layout_expectation "$expected_url" "$expectation" "$panes_file"; then
        echo "native-layout-unready: authoritative pane inventory does not prove expected $expectation identity" >&2
        rm -f "$attempt_file" "$stderr_file" "$validator_stderr" "$empty_file"
        return 1
    fi
    for attempt in 1 2 3; do
        command_status=0
        "$@" > "$attempt_file" 2> "$stderr_file" || command_status=$?
        if [ "$command_status" -ne 0 ]; then
            provenance="$(zaphod_bounded_reply_provenance "dump-layout attempt=$attempt/3" \
                "$command_status" "$attempt_file" "$stderr_file")"
            echo "native-layout-unready: $provenance" >&2
            rm -f "$attempt_file" "$stderr_file" "$validator_stderr" "$empty_file"
            return 1
        fi

        validator_status=0
        "$validator" "$attempt_file" "$expected_url" "$expectation" 2> "$validator_stderr" || validator_status=$?
        if [ "$validator_status" -eq 0 ]; then
            mv "$attempt_file" "$output_file"
            rm -f "$stderr_file" "$validator_stderr" "$empty_file"
            return 0
        fi

        provenance="$(zaphod_bounded_reply_provenance "dump-layout attempt=$attempt/3" \
            "$command_status" "$attempt_file" "$stderr_file"); \
$(zaphod_bounded_reply_provenance validator "$validator_status" "$empty_file" "$validator_stderr")"
        retryable=0
        if [ "$validator_status" -eq 20 ] && [ ! -s "$attempt_file" ]; then
            retryable=1
        elif [ "$expectation" = present ]; then
            if [ "$validator_status" -eq 21 ]; then
                retryable=1
            elif [ "$validator_status" -eq 20 ]; then
                if jq -e 'type == "array"' "$attempt_file" >/dev/null 2>&1; then
                    retryable=1
                fi
            fi
        fi
        if [ "$retryable" -eq 1 ] && [ "$attempt" -lt 3 ]; then
            echo "transient-native-layout-reply: $provenance" >&2
            sleep 0.05
            continue
        fi
        echo "native-layout-unready: $provenance" >&2
        rm -f "$attempt_file" "$stderr_file" "$validator_stderr" "$empty_file"
        return 1
    done
    return 1
}

zaphod_render_layout() {
    local template="$1"
    local wasm_url="$2"
    local output="$3"
    local recipient_token="${4:-}" replacement token_replacement
    replacement="${wasm_url//&/\\&}"
    replacement="${replacement//|/\\|}"
    token_replacement="${recipient_token//&/\\&}"
    token_replacement="${token_replacement//|/\\|}"
    if [ -n "$recipient_token" ]; then
        sed -e "s|__ZAPHOD_WASM__|$replacement|g" \
            -e "s|__ZAPHOD_RECIPIENT__|$token_replacement|g" "$template" > "$output"
    else
        sed -e "s|__ZAPHOD_WASM__|$replacement|g" \
            -e '/__ZAPHOD_RECIPIENT__/d' "$template" > "$output"
    fi
}

zaphod_kdl_escape() {
    local value="$1"
    value="${value//\\/\\\\}"
    value="${value//\"/\\\"}"
    printf '%s\n' "$value"
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
            sub(/[[:space:]]+\/\/.*/, "", line)
            if (line ~ /^[[:space:]]*\/\//) {
                next
            }
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

zaphod_layout_plugin_entries() {
    local layout_file="$1"
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
            sub(/[[:space:]]+\/\/.*/, "", line)
            if (line ~ /^[[:space:]]*\/\//) {
                next
            }
            opens = gsub(/\{/, "{", line)
            closes = gsub(/\}/, "}", line)
            if (!in_plugin && match(line, /plugin[[:space:]]+location="[^"]+"/)) {
                plugin_url = substr(line, RSTART, RLENGTH)
                sub(/^plugin[[:space:]]+location="/, "", plugin_url)
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
    ' "$layout_file"
}

zaphod_validate_layout_identity() {
    local layout_file="$1"
    local expected_url="$2"
    local entries url has_rail errors
    entries="$(zaphod_layout_plugin_entries "$layout_file")" || return 1
    if [ -z "$entries" ]; then
        echo "no Zaphod plugin panes found in $layout_file" >&2
        return 1
    fi
    errors=0
    while IFS="$(printf '\t')" read -r url has_rail; do
        if [ "$url" != "$expected_url" ]; then
            echo "Zaphod layout URL mismatch: $url" >&2
            echo "expected: $expected_url" >&2
            errors=1
        fi
        if [ "$has_rail" != "1" ]; then
            echo "Zaphod layout plugin missing rail \"1\": $url" >&2
            errors=1
        fi
    done <<EOF
$entries
EOF
    [ "$errors" -eq 0 ]
}

zaphod_cleanup_live_validation() {
    if [ -n "$ZAPHOD_LIVE_SESSION_NAME" ]; then
        zellij delete-session --force "$ZAPHOD_LIVE_SESSION_NAME" >/dev/null 2>&1 || true
        ZAPHOD_LIVE_SESSION_NAME=""
    fi
    if [ -n "$ZAPHOD_LIVE_DATA_ROOT" ] && [ -d "$ZAPHOD_LIVE_DATA_ROOT" ]; then
        rm -rf "$ZAPHOD_LIVE_DATA_ROOT"
        ZAPHOD_LIVE_DATA_ROOT=""
    fi
}

zaphod_validate_layout_live() {
    local layout_file="$1"
    local config_root="$2"
    local config_file="$3"
    local expected_url="$4"
    local data_root session_name dump_file status
    data_root="$(mktemp -d "${TMPDIR:-/tmp}/zaphod-layout-check.XXXXXX")" || return 1
    session_name="zlc-$$-${RANDOM:-0}"
    dump_file="$data_root/live-layout.kdl"
    ZAPHOD_LIVE_DATA_ROOT="$data_root"
    ZAPHOD_LIVE_SESSION_NAME="$session_name"
    status=0

    zellij --config-dir "$config_root" --config "$config_file" --data-dir "$data_root" \
        --layout "$layout_file" attach "$session_name" --create-background >/dev/null 2>&1 || status=$?
    if [ "$status" -eq 0 ]; then
        ZELLIJ_SESSION_NAME="$session_name" \
            zellij --config-dir "$config_root" --config "$config_file" --data-dir "$data_root" \
                action dump-layout > "$dump_file" || status=$?
    fi
    if [ "$status" -eq 0 ]; then
        zaphod_validate_layout_identity "$dump_file" "$expected_url" || status=$?
    fi

    zaphod_cleanup_live_validation
    if [ "$status" -ne 0 ]; then
        echo "live Zellij layout validation failed for $layout_file" >&2
        return "$status"
    fi
}
