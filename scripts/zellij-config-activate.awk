# ABOUTME: Safely rewrites existing Zaphod MessagePlugin routes in a Zellij KDL config.
# ABOUTME: Adds native Alt Shift z NewTab bindings only in scopes that already own Zaphod.

function die(message) {
    print "zaphod config activation: " message > "/dev/stderr"
    failed = 1
    exit 1
}

function code_before_comment(line,    i, character, next_character, quoted, escaped, output) {
    quoted = 0
    escaped = 0
    output = ""
    for (i = 1; i <= length(line); i++) {
        character = substr(line, i, 1)
        next_character = substr(line, i + 1, 1)
        if (quoted) {
            output = output character
            if (escaped) {
                escaped = 0
            } else if (character == "\\") {
                escaped = 1
            } else if (character == "\"") {
                quoted = 0
            }
            continue
        }
        if (character == "\"") {
            quoted = 1
            output = output character
            continue
        }
        if (character == "/" && next_character == "/") {
            break
        }
        output = output character
    }
    return output
}

function brace_delta(line,    code, i, character, delta) {
    code = code_before_comment(line)
    delta = 0
    for (i = 1; i <= length(code); i++) {
        character = substr(code, i, 1)
        if (character == "{") {
            delta++
        } else if (character == "}") {
            delta--
        }
    }
    return delta
}

function leading_whitespace(line,    match_length) {
    match(line, /^[ \t]*/)
    match_length = RLENGTH
    return substr(line, 1, match_length)
}

function is_keybinds_start(code) {
    return code ~ /^[ \t]*keybinds([ \t{]|$)/
}

function is_scope_start(code) {
    return code ~ /^[ \t]*(normal|locked|pane|tab|resize|move|scroll|search|entersearch|renametab|renamepane|session|tmux|shared|shared_except|shared_among)([ \t\"]|\{|$)/
}

function is_alt_shift_z_bind(code) {
    return code ~ /^[ \t]*bind[ \t]+"Alt Shift z"([ \t{;]|$)/
}

function is_zaphod_message_plugin(code) {
    return code ~ /MessagePlugin[ \t]+"[^"]*zellij-sidebar\.wasm([^"]*)?"/
}

function is_rail_entry(code) {
    return code ~ /^[ \t]*rail[ \t]+"[^"]*"[ \t;]*$/
}

function replace_zaphod_url(line,    first_quote, after_first_quote, second_quote_relative, second_quote) {
    first_quote = index(line, "\"")
    if (first_quote == 0) {
        die("Zaphod MessagePlugin has no quoted URL")
    }
    after_first_quote = substr(line, first_quote + 1)
    second_quote_relative = index(after_first_quote, "\"")
    if (second_quote_relative == 0) {
        die("Zaphod MessagePlugin has an unterminated URL")
    }
    second_quote = first_quote + second_quote_relative
    return substr(line, 1, first_quote) wasm_url substr(line, second_quote)
}

function replace_rail(line,    indent) {
    indent = leading_whitespace(line)
    return indent "rail \"1\""
}

function clear_scope_buffer(    line_number) {
    for (line_number = 1; line_number <= scope_line_count; line_number++) {
        delete scope_lines[line_number]
    }
    scope_line_count = 0
    scope_active = 0
}

function emit_native_keybind(indent) {
    print indent "bind \"Alt Shift z\" {"
    print indent "    NewTab { layout \"zaphod\"; }"
    print indent "}"
}

function emit_scope(    line_number, line, code, delta, scope_has_zaphod, zaphod_count, alt_shift_z_count, plugin_active, plugin_depth, plugin_rail_seen, plugin_indent, skip_bind, skip_bind_depth, target_indent) {
    scope_has_zaphod = 0
    zaphod_count = 0
    alt_shift_z_count = 0
    plugin_active = 0
    plugin_depth = 0

    for (line_number = 1; line_number <= scope_line_count; line_number++) {
        line = scope_lines[line_number]
        code = code_before_comment(line)
        delta = brace_delta(line)
        if (plugin_active) {
            plugin_depth += delta
            if (plugin_depth < 0) {
                die("malformed Zaphod MessagePlugin block")
            }
            if (plugin_depth == 0) {
                plugin_active = 0
            }
            continue
        }
        if (is_zaphod_message_plugin(code)) {
            scope_has_zaphod = 1
            zaphod_count++
            plugin_depth = delta
            if (plugin_depth <= 0) {
                die("Zaphod MessagePlugin must use a multiline block")
            }
            plugin_active = 1
        }
        if (is_alt_shift_z_bind(code)) {
            alt_shift_z_count++
        }
    }
    if (plugin_active) {
        die("unterminated Zaphod MessagePlugin block")
    }

    if (!scope_has_zaphod) {
        if (alt_shift_z_count > 0) {
            die("Alt Shift z is already bound outside a Zaphod keybind scope")
        }
        for (line_number = 1; line_number <= scope_line_count; line_number++) {
            print scope_lines[line_number]
        }
        return
    }
    if (alt_shift_z_count > 1) {
        die("multiple Alt Shift z bindings in one Zaphod keybind scope")
    }

    routed_scope_count++
    target_indent = leading_whitespace(scope_lines[1]) "    "
    plugin_active = 0
    plugin_depth = 0
    plugin_rail_seen = 0
    plugin_indent = ""
    skip_bind = 0
    skip_bind_depth = 0

    for (line_number = 1; line_number <= scope_line_count; line_number++) {
        line = scope_lines[line_number]
        code = code_before_comment(line)
        delta = brace_delta(line)

        if (line_number == scope_line_count) {
            emit_native_keybind(target_indent)
            print line
            continue
        }

        if (skip_bind) {
            skip_bind_depth += delta
            if (skip_bind_depth < 0) {
                die("malformed Alt Shift z binding")
            }
            if (skip_bind_depth == 0) {
                skip_bind = 0
            }
            continue
        }
        if (is_alt_shift_z_bind(code)) {
            if (delta < 0) {
                die("malformed Alt Shift z binding")
            }
            if (delta > 0) {
                skip_bind = 1
                skip_bind_depth = delta
            }
            continue
        }

        if (plugin_active) {
            if (is_rail_entry(code)) {
                line = replace_rail(line)
                plugin_rail_seen = 1
            }
            plugin_depth += delta
            if (plugin_depth < 0) {
                die("malformed Zaphod MessagePlugin block")
            }
            if (plugin_depth == 0 && !plugin_rail_seen) {
                print plugin_indent "    rail \"1\""
            }
            print line
            if (plugin_depth == 0) {
                plugin_active = 0
            }
            continue
        }

        if (is_zaphod_message_plugin(code)) {
            line = replace_zaphod_url(line)
            plugin_depth = brace_delta(line)
            if (plugin_depth <= 0) {
                die("Zaphod MessagePlugin must use a multiline block")
            }
            plugin_active = 1
            plugin_rail_seen = 0
            plugin_indent = leading_whitespace(line)
            print line
            continue
        }

        print line
    }
}

{
    line = $0
    code = code_before_comment(line)
    delta = brace_delta(line)

    if (!in_keybinds) {
        if (document_depth == 0 && is_keybinds_start(code)) {
            if (keybinds_seen) {
                die("multiple keybinds blocks are unsupported")
            }
            keybinds_seen = 1
            keybinds_outer_depth = document_depth
            if (delta <= 0) {
                die("keybinds block must open on its declaration line")
            }
            in_keybinds = 1
            print line
            document_depth += delta
            next
        }
        print line
        document_depth += delta
        next
    }

    if (scope_active) {
        scope_lines[++scope_line_count] = line
        scope_depth += delta
        document_depth += delta
        if (scope_depth < scope_outer_depth) {
            die("malformed keybind scope")
        }
        if (scope_depth == scope_outer_depth) {
            emit_scope()
            clear_scope_buffer()
        }
        next
    }

    if (document_depth == keybinds_outer_depth + 1 && is_scope_start(code)) {
        scope_active = 1
        scope_outer_depth = document_depth
        scope_depth = document_depth + delta
        if (scope_depth <= scope_outer_depth) {
            die("keybind scope must use a multiline block")
        }
        scope_lines[++scope_line_count] = line
        document_depth += delta
        next
    }

    if (is_zaphod_message_plugin(code)) {
        die("Zaphod MessagePlugin must be inside a direct keybind scope")
    }
    if (is_alt_shift_z_bind(code)) {
        die("Alt Shift z must be inside a direct keybind scope")
    }
    print line
    document_depth += delta
    if (document_depth < keybinds_outer_depth) {
        die("malformed keybinds block")
    }
    if (document_depth == keybinds_outer_depth) {
        in_keybinds = 0
    }
}

END {
    if (failed) {
        exit 1
    }
    if (!keybinds_seen) {
        die("no keybinds block found")
    }
    if (in_keybinds || scope_active) {
        die("unterminated keybinds block")
    }
    if (routed_scope_count == 0) {
        die("no Zaphod MessagePlugin route found")
    }
}
