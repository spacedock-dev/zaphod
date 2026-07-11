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
sprint: s1-trusted-test-profile-onramp
sprint-lane:
group: contingent-enablement
sprint-readiness: defer
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

Keep native code in the existing Go module, `grout/go.mod`, with three
non-overlapping boundaries:

- `grout/cmd/zaphod` owns process startup, stdin/stdout, exit
  classification, and the one-line handshake only.
- `grout/internal/zaphodcli` owns typed `CommandEnvelope` decoding,
  protocol/correlation normalization, and exactly one typed dispatch to a
  service.
- `grout/internal/bindingcore` owns `BindingV1`, registry locks and atomic
  writes, canonical-root resolution, reverse uniqueness, inventory
  classification, recovery policy, and its injected `Driver` interface
  through a `Service`.

This keeps the command independent of the Rust/WASM plugin and the current
`grout` row-emitter `main` package. Neither shell package may open the
registry, infer an identity from a name or active client, choose a repair, or
retry an indeterminate mutation. When implemented, the five `binding.*`
discriminants dispatch once to the corresponding `bindingcore.Service`
method; until then the skeleton returns `Unsupported` without dispatch.
No existing grout row, pipe, or Zellij behavior is imported into the CLI.
There is no native-CLI pure function to extend: `grout/rows.go` is
deliberately row-protocol-specific. The new pure boundaries are isolated
envelope validation/normalization in `zaphodcli` and registry/recovery
transitions in `bindingcore`.

Add `scripts/build-zaphod.sh` to build this command atomically. Its default
output is the ignored worktree-local artifact `target/zaphod/zaphod`; an
explicit `--output PATH` supports the lease-scoped candidate at
`$PROFILE_ROOT/bin/zaphod`. It uses the Go tool chain only, injects a
nonempty CLI version, and never calls `install.sh`, `zellij`, or writes a
user configuration directory. `install.sh` remains the sole global installer
and gains no native-binary copy/PATH behavior in this sprint.

The candidate is leased through the immutable, test-only `ProfileLeaseV1`
published by `foreground-attached-client-profile` at
`$PROFILE_ROOT/profile-lease-v1.json` only after its primary foreground
client is ready. The profile harness supplies the exact lease
`profile_root` and uses
`scripts/build-zaphod.sh --output "$PROFILE_ROOT/bin/zaphod"`; it may print
`ZAPHOD_BIN` as a convenience, but the lease is the ownership contract.
The CLI creates no Zellij client, owns neither base nor secondary-client
teardown, and never derives namespace or session identity from a path, display
name, active client, or cwd. If a future typed command carries a marker tuple,
each value remains an opaque typed field; the CLI does not interpret it as
identity. This is test wiring only: it adds no layout reference, keybinding,
or Zellij invocation.

### Versioned native seam

`zaphod protocol` writes exactly one JSON `Handshake` line:

```json
{"protocol_version":1,"cli_version":"0.1.0-dev","capabilities":["protocol.handshake.v1","protocol.invoke.v1","health.v1"]}
```

`protocol_version` is a wire-compatibility major, not the binary release
version. A caller compares it and its required capabilities before invoking a
command; an absent capability means no fallback or implicit success. All
`Handshake`, `CommandEnvelope`, and `ResultEnvelope` values use
`protocol_version: 1`.

`zaphod internal invoke` accepts one JSON
`CommandEnvelope { protocol_version, request_id, command }` on stdin and
writes exactly one correlated
`ResultEnvelope { protocol_version, request_id, outcome }` on stdout.
`request_id` is a nonempty opaque value copied verbatim when it can be
decoded; `command` is a tagged, typed union, never a shell string or
free-form argv. The initial handshake advertises exactly
`protocol.handshake.v1`, `protocol.invoke.v1`, and `health.v1`. It
advertises no `binding.*` or `driver.*` capability until the corresponding
`bindingcore.Service` or driver capability is integrated and process-proved.

The envelope fixtures and implementation use this exact
`ErrorCodeV1`/mutation matrix:

| Boundary condition | Required v1 result |
| --- | --- |
| Invalid JSON or a missing/wrong required envelope field | `MalformedEnvelope`, one result under the decoded nonempty request ID or ``, `Unchanged`, and no dispatch. |
| Request or handshake protocol major is not 1 | `ProtocolMismatch`, one result under the decoded/sent request ID, `Unchanged`, and no dispatch. |
| A caller receives a parseable response whose ID differs from its sent ID | Normalize locally to `CorrelationMismatch` under the sent ID, `Unchanged`; ignore the response value, do not retry, and do not call a driver. |
| Unknown command or a missing advertised requirement | `Unsupported`, one correlated `Unchanged` failure with no dispatch or fallback. |

`Success` uses `Changed` only after a mutation is observed complete and
durable; it uses `Unchanged` for a read, reuse, or no-op. A failure is
`Unchanged` unless a native mutation was issued and completion cannot be
observed; only that case may be `Indeterminate`, which requires fresh
`binding.inspect` before any retry and never permits a name-based second
create. The skeleton implements only side-effect-free `health`, returning
the handshake with `Unchanged`. It may decode known binding discriminants
only to return `Unsupported` until the binding core is linked. No command
resolves a display name, active client, shell command, or Zellij state.

### Documentation change proposed

Add a short **Native CLI candidate** section to `README.md` showing
`./scripts/build-zaphod.sh` and `./target/zaphod/zaphod protocol`. It will
state that a disposable profile exposes its leased candidate only at
`$PROFILE_ROOT/bin/zaphod`, after `ProfileLeaseV1` is ready, and that Sprint
1 creates no installed command, Zellij action, layout, client, or global
configuration. That makes the direct inspection path discoverable without
promising managed-view or binding behavior.

## Acceptance criteria

### Offline

**AC-1 — A clean worktree owns one executable, versioned native interface.**
`scripts/build-zaphod.sh` creates exactly one executable at the documented
default path, and `zaphod protocol` returns one parseable handshake with
`protocol_version == 1`, a nonempty CLI version, and exactly
`protocol.handshake.v1`, `protocol.invoke.v1`, and `health.v1`. It
advertises no `binding.*` or `driver.*` token. The expected identity comes
from the shared Sprint 1 contract, not generated source text.

Verified by: an external process test builds from a fresh worktree, parses the
binary's stdout as JSON, checks line count and the exact capability set, and
runs `go test ./...` plus `go vet ./...` from `grout`.

**AC-2 — Typed invocation is correlated and fail-closed before native behavior
exists.** For `health`, `internal invoke` returns one result whose protocol
and request ID exactly match the request and whose mutation is `Unchanged`.
Malformed input, a protocol-major mismatch, a correlation mismatch, and an
unavailable binding command produce exactly `MalformedEnvelope`,
`ProtocolMismatch`, `CorrelationMismatch`, and `Unsupported`,
respectively, with the matrix's request-ID and `Unchanged` rules. They never
fall back to display names, active clients, shell execution, or a Zellij
command.

Verified by: black-box stdin/stdout tests exercise valid, malformed,
unsupported, incompatible, and mismatched-correlation fixtures; a fake
`zellij` on `PATH` fails the test if the skeleton attempts to invoke it.

**AC-3 — Candidate-profile ownership follows its lease and is recoverable.**
After the profile packet publishes its immutable `ProfileLeaseV1`, the
candidate exists only at `$PROFILE_ROOT/bin/zaphod` under that lease's exact
root; its handshake works while the lease is alive. It creates no base or
secondary Zellij client, and owns no client or profile teardown. On profile
normal exit or `INT`/`TERM`/`HUP`, the temporary root and its binary
disappear, the profile session is absent, and SHA-256 states of the standing
config and `layouts/zaphod.kdl` equal their independently captured baselines.

Verified by: extend the existing process-level disposable-profile regression
to consume the lease's exact root, run the candidate, record that it creates
no extra client/process group, then exercise normal and signal cleanup paths
and compare pre/post file states. This is an automated offline-style
regression, but it is not scheduled until the foreground-profile validation
gate passes.

### Interactive

**AC-4 — A person can inspect the candidate without changing their
multiplexer.** After the foreground attached-client profile passes its gate,
CL can use the ready lease's `$PROFILE_ROOT/bin/zaphod protocol` inside that
disposable profile, see the one-line handshake, keep the terminal canary
responsive, and exit with no changed standing Zellij files. No keybinding,
pane, tab, layout, client, or controller action is expected.

Verified by: the post-gate disposable-profile drill uses the existing
foreground canary and independent before/after global-file hashes; CL observes
the command and cleanup live.

## Test plan

**Riskiest unproven mechanism:** a profile that already builds a WASM candidate
can publish an immutable `ProfileLeaseV1`, create and expose a second native
executable only at that lease's root, and clean it up without weakening its
foreground-process or global-isolation guarantees. The first, smallest
end-to-end invalidator is therefore: after the foreground-profile gate, start
a candidate profile under the existing process harness, wait for its ready
lease, run `"$PROFILE_ROOT/bin/zaphod" protocol`, verify it created no
secondary client, then terminate the profile and prove the binary/root/session
are gone while the pre/post standing-file hashes match. It does not exercise a
key, managed tab, binding service, or Zellij driver.

1. Add pure Go tests for handshake validation, capability membership, tagged
   command decoding, response-correlation normalization, and every exact
   `ErrorCodeV1`/mutation row.
2. Add black-box binary tests for one-line stdout, request-ID correlation,
   exit classification, exact capability advertisement, and
   malformed/unknown/incompatible input.
3. Add the profile process regression above only after the foreground-profile
   task validates; consume `ProfileLeaseV1`, run normal, `INT`, `TERM`,
   and `HUP` cleanup cases, and prove that the CLI created no client.
4. Run `go test ./...` and `go vet ./...` in `grout` plus the unchanged Rust
   suite, followed by the bounded CL inspection drill for AC-4.

## Out of scope

- Any managed-view controller, Zellij/tmux command, keybinding, tab creation,
  layout change, pane adoption, or native-ID decision.
- Binding storage, reverse uniqueness, repair policy, driver implementation,
  or implementation of binding commands; those belong to
  `managed-view-driver-contract` after the shared contract freeze.
- Hub, dock, provider, `notify`, grout row behavior, installer rollout, PATH
  install, global configuration mutation, or production release packaging.

## Stage Report: ideation

- DONE: Choose the smallest native zaphod package owner, artifact path, build/test-profile wiring, and isolation boundary without touching standing installation or global multiplexer state.
  State commit `86ddb5f` selects `grout/cmd/zaphod`, `target/zaphod/zaphod`, and a profile-private `ZAPHOD_BIN` with no installer or Zellij call.
- DONE: Define a version/capability handshake and typed command/result envelope that the shared binding contract can consume without managed-tab behavior.
  The packet and sibling contract agree on `Handshake`, `CommandEnvelope`, `ResultEnvelope`, request-ID echo, capability checks, and `Changed|Unchanged|Indeterminate` outcomes.
- DONE: Write bounded acceptance criteria and an offline-first test plan; exclude controller, keybinding, pane-adoption, hub, dock, and provider behavior.
  AC-1 through AC-3 are agent-reproducible; AC-4 and the smallest profile E2E check are explicitly gated on foreground-profile validation.

### Summary

The native artifact is a Go command isolated from the WASM plugin and grout's
row-emitter behavior, with an atomic candidate build and disposable-profile
ownership. The only unresolved risk is safely exposing that second artifact
through the existing foreground profile; its bounded check is deferred rather
than assumed.

## Stage Report: ideation (cycle 2)

- DONE: Align the Go ownership and command-dispatch design to grout/internal/bindingcore, grout/internal/zaphodcli, and grout/cmd/zaphod.
  The revised ownership boundary assigns registry/recovery policy to `bindingcore`, one typed envelope dispatch to `zaphodcli`, and process-only work to `cmd/zaphod`.
- DONE: Adopt the exact protocol-v1 capability, error, correlation, and mutation matrix without advertising binding or driver capabilities before integration/proof.
  The packet now requires the three-token initial handshake and the canonical `MalformedEnvelope`, `ProtocolMismatch`, `CorrelationMismatch`, `Unsupported`, and mutation semantics in fixtures and black-box checks.
- DONE: Specify candidate-binary use through ProfileLeaseV1 without creating clients, inferring identity, or owning lease teardown.
  The candidate is constrained to `$PROFILE_ROOT/bin/zaphod` under the ready immutable lease; profile ownership and client teardown remain with the foreground-profile packet.

### Summary

This alignment revision makes the native CLI a narrow transport shell over the
future binding core rather than a second policy owner. It freezes its initial
wire behavior and lease-scoped artifact ownership while keeping binding and
driver capabilities unadvertised until independently integrated and proved.

## Stage Report: ideation (cycle 3)

- DONE: Repair AC-1 evidence or explicitly defer it to implementation with a cited reason.
  Design evidence: `docs/roadmap.md:23-25,33-35,47-55,74-82`, `docs/plan-agent-rail.md:37-57`, and `grout/go.mod:1-5` establish the Sprint 1 native-artifact goal, Go/grout boundary, and post-contract implementation lane. The inspected tree has neither `grout/cmd/zaphod` nor `scripts/build-zaphod.sh`; fresh-worktree build, one-line handshake, exact capability set, `go test`, and `go vet` remain implementation-stage proof.
- DONE: Repair AC-3 evidence or explicitly defer it to implementation with a cited reason.
  Design evidence: `docs/roadmap.md:47-61` sequences foreground validation before native profile wiring; `docs/docking-approach.md:790-805`, `scripts/zellij-worktree-test-profile.sh:25-73`, and `tests/zellij-install-profile-test.sh:578-639` define the existing disposable-root, cleanup, and hash observables; `docs/agent-rail-dev/.spacedock-state/foreground-attached-client-profile.md:115-121` supplies the upstream ownership contract. These sources do not prove a new lease or candidate binary. That proof remains in the profile-wiring implementation lane: consume an actual `ProfileLeaseV1`, run `$PROFILE_ROOT/bin/zaphod`, observe no extra client, and exercise normal, `INT`, `TERM`, and `HUP` cleanup.
- DONE: Append a report-only ideation cycle; do not edit product files or redesign scope.
  Inspected sources are `/tmp/spacedock-dispatch/spacedock-ensign-zaphod-native-cli-skeleton-ideation.md`, this record's AC-1/AC-3 and test-plan sections, and the seven cited repository/state files; this cycle changes only this state record and preserves the approved ownership, protocol-v1, and `ProfileLeaseV1` direction.

### Summary

The report now distinguishes durable design inputs from behavior that only an
implemented CLI and profile integration can demonstrate. AC-1 and AC-3 retain
their required executable proofs, deferred to the lanes that can produce them;
the CLI remains process-only and lease-scoped.

### AC evidence and deferred-proof summary

- AC-1 — Design basis: roadmap, Go-module, and grout-boundary citations above. Deferred proof: fresh-worktree binary build, exact `protocol` JSON/capabilities, `go test ./...`, and `go vet ./...` after the CLI exists.
- AC-3 — Design basis: roadmap sequencing, existing profile lifecycle observables, and the foreground packet's ownership boundary. Deferred proof: real ready-lease consumption and normal/signal cleanup of the candidate profile after foreground validation.
