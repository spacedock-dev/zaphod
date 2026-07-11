---
id: bc7r9aj9s4a799dxt0qvebfq
title: Native zaphod CLI skeleton and artifact ownership
status: ideation
source: managed-view roadmap Sprint 1 foundation, senior staff review 2026-07-11
started: 2026-07-11T05:21:24Z
completed:
verdict:
score: 0.98
worktree:
issue:
pr:
mod-block:
---

## Problem

The managed-workspace roadmap needs one local `zaphod` executable before the
binding core or a Zellij feasibility harness can invoke native work safely.
Today the repository owns a Rust/WASM sidebar and a Go `grout` module, but
neither owns a versioned native command, its artifact lifecycle, or a
machine-readable result boundary. Folding a native binary into the WASM crate
would couple host behavior to `zellij-tile`; putting transport in the existing
grout root package would couple it to the row-emitter executable.

## Proposed approach

### Ownership and artifact boundary

Keep native code in the existing Go module, `grout/go.mod`, but give it its
own package boundary: public command source at `grout/cmd/zaphod` and pure
wire/dispatch code at `grout/internal/zaphodcli`. This is smaller and more
coherent than adding a second root Go module, while keeping it independent of
both the Rust/WASM plugin and the current `grout` row-emitter `main` package.
No existing grout row, pipe, or Zellij behavior is imported into the CLI.
There is no native CLI pure function to extend: `grout/rows.go` is deliberately
row-protocol-specific. The implementation starts with isolated pure
`Handshake` validation, `CommandEnvelope` decoding, and outcome construction
in `grout/internal/zaphodcli`, then gives the command a thin stdin/stdout
adapter.

Add `scripts/build-zaphod.sh` to build this command atomically. Its default
output is the ignored worktree-local artifact `target/zaphod/zaphod`; an
explicit `--output PATH` supports a disposable-profile artifact. It uses the
Go tool chain only, injects a nonempty CLI version, and never calls
`install.sh`, `zellij`, or writes a user configuration directory.
`install.sh` remains the sole global installer and gains no native-binary
copy/PATH behavior in this sprint.

The existing `scripts/zellij-worktree-test-profile.sh` will create its
temporary root before candidate builds, call
`scripts/build-zaphod.sh --output "$PROFILE_ROOT/bin/zaphod"`, and print
`ZAPHOD_BIN=` beside its current metadata. The binary therefore belongs to the
profile and disappears with it. This is test wiring only: it adds no layout
reference, keybinding, or Zellij invocation.

### Versioned native seam

`zaphod protocol` writes exactly one JSON `Handshake` line:

```json
{"protocol_version":1,"cli_version":"0.1.0-dev","capabilities":["protocol.handshake.v1","protocol.invoke.v1","health.v1"]}
```

`protocol_version` is a wire-compatibility major, not the binary release
version. A caller compares it and its required capabilities before invoking a
command; an absent capability means no fallback or implicit success.

`zaphod internal invoke` accepts one JSON `CommandEnvelope` on stdin and
writes exactly one correlated `ResultEnvelope` on stdout. The shared names and
semantics, coordinated with the parallel binding-contract packet, are:

- `Handshake { protocol_version, cli_version, capabilities }` identifies the
  artifact and only capabilities it actually implements.
- `CommandEnvelope { protocol_version, request_id, command }` has a nonempty
  opaque `request_id` copied verbatim to the response. `command` is a tagged,
  typed union, never a shell string or free-form argv.
- `ResultEnvelope { protocol_version, request_id, outcome }` is emitted once
  for every stdin input line. If a request ID cannot be decoded, it uses an
  empty ID and a typed input failure; stderr remains diagnostic only.
- `Outcome<T>` is either `Success { value, mutation: Changed|Unchanged }` or
  `Failure { error, mutation: Unchanged|Indeterminate, diagnostic? }`.
  `Indeterminate` is reserved for a future issued-native-mutation whose
  completion cannot be observed; this skeleton's failures are `Unchanged`.

The skeleton implements only side-effect-free `health`, which returns the
handshake with `Unchanged`, and parses `binding.inspect` only to return typed
`Unsupported` until the binding core owns behavior. Future
`binding.unbind`, `binding.rebind`, and `binding.repair` variants are not
implemented or advertised here. No command resolves a display name, active
client, shell command, or Zellij state.

### Documentation change proposed

Add a short **Native CLI candidate** section to `README.md` showing
`./scripts/build-zaphod.sh` and `./target/zaphod/zaphod protocol`, and state
that Sprint 1 creates no installed command, Zellij action, layout, or global
configuration. That makes the direct inspection path discoverable without
promising managed-view behavior.

## Acceptance criteria

### Offline

**AC-1 — A clean worktree owns one executable, versioned native interface.**
`scripts/build-zaphod.sh` creates exactly one executable at the documented
default path, and `zaphod protocol` returns one parseable handshake with
`protocol_version == 1`, a nonempty CLI version, and the baseline protocol
capabilities. The expected identity comes from this shared Sprint 1 contract,
not generated source text.

Verified by: an external process test builds from a fresh worktree, parses the
binary's stdout as JSON, checks line count and fields, and runs `go test ./...`
plus `go vet ./...` from `grout`.

**AC-2 — Typed invocation is correlated and fail-closed before native behavior
exists.** For `health`, `internal invoke` returns one result whose protocol
and request ID exactly match the request and whose mutation is `Unchanged`.
A protocol-major mismatch, malformed envelope, or unavailable binding command
returns one typed failure/unsupported result and never falls back to display
names, active clients, shell execution, or a Zellij command.

Verified by: black-box stdin/stdout tests exercise valid, malformed,
unsupported, and incompatible fixtures; a fake `zellij` on `PATH` fails the
test if the skeleton attempts to invoke it.

**AC-3 — Candidate-profile ownership is isolated and recoverable.** The
worktree profile exposes executable `ZAPHOD_BIN` under its printed temporary
root; its handshake works while the profile is alive. On normal exit or
`INT`/`TERM`/`HUP`, the temporary root and its binary disappear, the profile
session is absent, and SHA-256 states of the standing config and
`layouts/zaphod.kdl` equal their independently captured baselines.

Verified by: extend the existing process-level disposable-profile regression
to run the printed candidate binary, then exercise normal and signal cleanup
paths and compare pre/post file states. This is an automated offline-style
regression, but it is not scheduled until the foreground-profile validation
gate passes.

### Interactive

**AC-4 — A person can inspect the candidate without changing their
multiplexer.** After the foreground attached-client profile passes its gate,
CL can run the printed `ZAPHOD_BIN protocol` inside that disposable profile,
see the one-line handshake, keep the terminal canary responsive, and exit with
no changed standing Zellij files. No keybinding, pane, tab, layout, or
controller action is expected.

Verified by: the post-gate disposable-profile drill uses the existing
foreground canary and independent before/after global-file hashes; CL observes
the command and cleanup live.

## Test plan

**Riskiest unproven mechanism:** a profile that already builds a WASM candidate
can safely create, expose, and clean up a second native executable without
weakening its foreground-process or global-isolation guarantees. The first,
smallest end-to-end invalidator is therefore: after the foreground-profile
gate, start a candidate profile under the existing process harness, wait for
`PROFILE_ROOT` and `ZAPHOD_BIN`, run `"$ZAPHOD_BIN" protocol`, then terminate
the profile and prove the binary/root/session are gone while the pre/post
standing-file hashes match. It does not exercise a key, managed tab, or
Zellij driver.

1. Add pure Go tests for handshake validation, capability membership, tagged
   command decoding, and each outcome/mutation state.
2. Add black-box binary tests for one-line stdout, request-ID correlation,
   exit classification, and malformed/unknown input.
3. Add the profile process regression above only after the foreground-profile
   task validates; run normal, `INT`, `TERM`, and `HUP` cleanup cases.
4. Run `go test ./...` and `go vet ./...` in `grout` plus the unchanged Rust
   suite, followed by the bounded CL inspection drill for AC-4.

## Out of scope

- Any managed-view controller, Zellij/tmux command, keybinding, tab creation,
  layout change, pane adoption, or native-ID decision.
- Binding storage, reverse uniqueness, repair policy, or implementation of
  binding commands; those belong to `managed-view-driver-contract` after the
  shared contract freeze.
- Hub, dock, provider, `notify`, grout row behavior, installer rollout, PATH
  install, global configuration mutation, or production release packaging.
