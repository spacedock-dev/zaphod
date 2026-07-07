---
title: Grout skeleton
status: validation
source: plan sprint 0 (docs/plan-agent-rail.md)
score: 0.8
id: fvfk1a5c1xcrbcjp6ap0z157
started: 2026-07-07T04:49:25Z
worktree: .worktrees/spacedock-ensign-grout-skeleton
---

## Problem

The walking skeleton needs the glue half: nothing today turns agentsview
sessions and subspace decision logs into rail rows. Grout (Go, new `grout/`
dir) is that binding — and sprint 0 only needs its thinnest thread.

## Proposed approach

Grout is the binding the plan's decisions pin: a Go binary (new `grout/`
directory) that turns agentsview session data and subspace decision logs into
typed rail rows and pipes them into the zellij session. Sprint 0 needs only
its thinnest thread: one one-shot run — fetch one session, read one gate
fixture, emit two rows, exit. The plugin never learns about agentsview; the
grout never learns about zellij beyond invoking `zellij pipe` (plan
decisions 1–2).

### Row protocol (pinned, plan decision 3)

One JSON object per line, one line per `zellij pipe` invocation, pipe name
`agent-event` (constant):

```
{"kind":"session","id":"…","cwd":"…","agent":"…","state":"…","summary":"…","ts":"2026-07-07T05:00:00Z"}
{"kind":"gate","log_path":"/abs/…/x.decisions.jsonl","workflow":"…","entity":"…","entity_title":"…","stage":"…","round":1,"recommendation":"…","ts":"2026-07-07T05:00:00Z"}
```

- Exactly these fields per kind, no extras; unknown-field tolerance is the
  plugin's concern (slice c).
- All values JSON strings except `round` (JSON number, the brief's
  `gate.round` int).
- `ts` is the emission timestamp, RFC3339 UTC. Source-side activity
  timestamps would be future additional fields, not overloads of `ts`.
- `summary` is clamped to 512 bytes (rune-safe truncation) so the worst-case
  row stays near ~1 KiB regardless of what the size probe finds.
- `log_path` is absolute (the plugin floats `subspace-tui` on it — slice c).
- `recommendation` carries the brief's `recommendation.verdict` string
  (`approve|revise|reject|hold`).

### Data sources

- **Session row:** one-shot `agentsview session get <id> --format json`;
  grout maps the response's id/cwd/agent/state/summary-bearing fields onto
  the row. Exact source field names are pinned at implementation time from a
  fixture recorded off the real binary (test plan item 3) — the row fields,
  not the source fields, are the contract.
- **Gate row:** one hardcoded gate log. Brief discovery inverts subspace's
  own derivation (`internal/brief/brief.go:175`, gate-foo.md →
  gate-foo.decisions.jsonl): brief path = TrimSuffix(log,
  ".decisions.jsonl") + ".md". Grout parses the brief frontmatter fields
  `gate.{workflow,entity,entity-title,stage,round}` and
  `recommendation.verdict` (shapes verified against subspace recon HEAD
  9be5fbc and the playground fixture). No folding: the skeleton emits the
  configured gate unconditionally; pending/resolved filtering is sprint 2,
  via the subspace binary.
- The playground fixture pair (`playground-gate.md` +
  `playground-gate.decisions.jsonl`) is vendored into `grout/testdata/` from
  `~/git/spacedock-research/spacedock-subspace` @ 9be5fbc so tests and the
  default config are machine-independent; the live demo may point at the
  real copy instead.

### Go module layout

```
grout/
  go.mod            module zaphod/grout; go 1.26; sole dep gopkg.in/yaml.v3
  main.go           hardcoded Config, run() wiring, exit codes
  rows.go           row structs, BuildSessionRow / BuildGateRow / clampSummary (pure)
  agentsview.go     FetchSession: exec `session get --format json`, decode
  gate.go           GateFromLog: brief-path derivation + frontmatter parse (pure given bytes)
  emit.go           EmitRow: zellij pipe with kill timer; pipeArgs (pure)
  *_test.go         beside each file
  testdata/         session-get.json (recorded), playground-gate.{md,decisions.jsonl} (vendored)
```

Single `package main` — the skeleton is one binary; sprint 1 splits packages
when SSE lands. Row/gate/argv construction are pure functions carrying over
the decide_toggle test discipline (SPEC "if building v2" #1). No existing
zaphod pure functions are extended — grout is a new module and the plugin
side belongs to slice (c).

### Hardcoded config

```go
type Config struct {
    AgentsviewBin     string        // "agentsview"
    SessionID         string        // default demo session; argv[1] overrides
    GateLog           string        // default grout/testdata/playground-gate.decisions.jsonl; argv[2] overrides
    ZellijBin         string        // "zellij"
    ZellijSession     string        // "" = inherit env; non-empty sets ZELLIJ_SESSION_NAME on the child
    PipeName          string        // "agent-event" — protocol constant
    PipeTimeout       time.Duration // 5 * time.Second — kill timer
    SummaryClampBytes int           // 512
}
```

Invocation `grout [session-id [gate-log]]` — two optional positionals,
because a hardcoded session id is stale by demo time; everything else stays
literal in `main.go`. A config file is sprint 2 (plan decision 4's glob
config).

### Emit semantics — fire-and-forget with a kill timer

- One process per row: `zellij pipe --name agent-event -- <json line>` via
  `exec.CommandContext` under `context.WithTimeout(PipeTimeout)`, with
  `cmd.WaitDelay = 2s` so Wait returns even against held pipes. Payload is a
  single argv token (ARG_MAX-bounded; rows are ~1 KiB).
- **Never `--plugin`/`--plugin-configuration`:** with `--plugin`, a
  non-running plugin gets launched — the SPEC #3 background-zombie class.
  Broadcast by name only; a rail-less session drops the row harmlessly.
- Rows emit sequentially, session then gate (order pinned for test
  determinism); each row is independent — a timed-out session-row pipe must
  not stop the gate row.
- On timeout the child is killed and the failure logged to stderr
  (`pipe timeout after 5s: kind=session`); no retry, no blocking beyond
  budget — the kill timer is the "forget". This protects grout from the
  wedged-instance hang the spike measured (13s+); the sibling
  plugin-pipe-unblock slice fixes the plugin side, grout defends
  independently.
- Exit code: 0 when every row's pipe exited 0 within budget; 1 otherwise —
  visible-not-blocking, scriptable.

### Riskiest unproven mechanism

**The pipe payload size ceiling.** The spike proved the transport (daemon →
grout → `zellij pipe` → rendered row, 1–6s) but never at size; if the
CLI→server→plugin path drops or truncates payloads above some small ceiling,
JSON-lines-with-summaries is the wrong protocol and this design is invalid.
The smallest end-to-end check is test plan item 1, listed first. Everything
else rides on proven mechanisms: the transport (2026-07-07 spike), the
brief/log shapes (subspace recon @9be5fbc), the `session get --format json`
surface (present in both PATH v0.32.1 and the plan-pinned v0.36.1).

### Environment notes (surfaced, not silently depended on)

- PATH `agentsview` on this machine reports **v0.32.1**; the plan pins
  v0.36.1 (the spike's). `session get <id> --format json` exists in both;
  the fixture-recording step records which version produced the fixture.
- The agent sandbox (safehouse) blocks `~/.agentsview` and
  `~/git/agentsview`, so agents cannot run agentsview live — fixture
  recording is a declared one-liner for CL (or any unsandboxed shell).
  Offline tests use only the fixture plus test-written fake binaries.

### Doc diff (proposed)

New `grout/README.md` (~30 lines): what grout is (one paragraph pointing at
plan decisions 1–4); skeleton usage (`go run ./grout [session-id [gate-log]]`
from inside the target zellij session); the two row-kind example lines;
failure posture (exit codes, 5s kill timer, the no-`--plugin` rule); fixture
provenance and the recording command. No SPEC.md change — this slice touches
no plugin behavior.

## Acceptance criteria

Offline (agent-reproducible):

**AC-1 — Value, wire-level: real source data crosses to the pipe as both row
kinds.** A grout run against the recorded agentsview fixture and the vendored
playground gate fixture produces exactly two `zellij pipe` invocations on
pipe name `agent-event`: one `kind:"session"` line whose
id/cwd/agent/state/summary equal the recorded fixture's values, and one
`kind:"gate"` line whose
workflow/entity/entity_title/stage/round/recommendation equal the playground
brief's frontmatter (`demo` / `playground` / `Playground handoff demo` /
`review` / 1 / `approve`) and whose log_path is the configured log's absolute
path. Count plus field-value baselines live outside grout's source (the
fixtures are recorded/vendored from real tools).
Verified by: `cd grout && go test -run TestEmitEndToEnd ./...` — a
test-written fake `zellij` script (wired via Config.ZellijBin) records argv;
the test asserts invocation count, pipe name, absence of `--plugin`, and
payload fields against the fixtures.

**AC-2 — Protocol pinned exactly to plan decision 3.** Every emitted line
unmarshals as a JSON object whose key set is exactly the plan's field set for
its kind (session: kind,id,cwd,agent,state,summary,ts; gate:
kind,log_path,workflow,entity,entity_title,stage,round,recommendation,ts),
with `round` a JSON number, `ts` RFC3339 UTC, and `summary` ≤512 bytes even
when the source summary is larger. Expected key sets are transcribed from
`docs/plan-agent-rail.md` decision 3 — outside the file under test.
Verified by: `cd grout && go test -run TestRowProtocol ./...`.

**AC-3 — A wedged pipe cannot wedge grout.** With a fake `zellij` that never
exits, a grout run terminates within 2×PipeTimeout+2s wall, leaves no child
process behind, writes the timeout reason to stderr, exits 1, and still
attempted the second row — timing, process-liveness, and exit-code baselines.
Verified by: `cd grout && go test -run TestPipeKillTimer ./...` — fake sleeps
300s; the test asserts elapsed time, that the recorded child pid is gone
(`kill -0` fails), and that both invocations were attempted.

Interactive (settled only by CL's live demo):

**AC-4 — Value, plugin-observable: both row kinds observable at the plugin
within 10 seconds of emit.** In CL's fresh zellij session with the sprint-0
plugin (pipe-unblock landed; rendered observation once slice (c) lands), one
grout run puts both rows at the plugin within 10s (spike baseline 1–6s). This
is the skeleton's half of the sprint-0 exit demo.
Verified by: CL runs grout inside the session and watches the rail — or,
pre-slice-(c), the plugin's `pipe recv name=agent-event` debug trace in the
server log.

**AC-5 — Measured payload ceiling on the record, above worst case.** The
agent-event pipe path demonstrably carries a single payload ≥4 KiB (≈4× the
clamped worst-case row), and the measured ceiling (or "≥256 KiB — unbounded
for our purposes") is recorded in this entity. A ceiling below ~1 KiB
invalidates the protocol → back to ideation.
Verified by: test plan item 1's probe, run live while the skeleton is up;
result appended to this file.

## Test plan

1. **Payload-size probe — first; failure invalidates the design.** In a live
   (or ztest bench) session running the rail with the `debug "1"` config
   key: `zellij pipe --name agent-event -- "$(python3 -c 'print("x"*4096)')"`,
   then 64 KiB, then 256 KiB; receipt confirmed per size via the plugin's
   `pipe recv name=agent-event` trace in the server log
   (`$TMPDIR/zellij-<uid>/zellij-log/zellij.log`, SPEC #21). Fallback
   observation without the debug key: the same sizes on pipe name `toggle` —
   the rail visibly toggling proves the payload traversed. Minutes of cost;
   piggybacks on any live session; ceiling recorded in this entity (AC-5).
2. **Offline TDD suite** (red-first per the implementation stage):
   `TestRowProtocol` (AC-2: key sets, round type, ts format, clamp),
   `TestGateRowFromBrief` (frontmatter → row against the vendored brief;
   derivation gate-foo.decisions.jsonl → gate-foo.md),
   `TestSessionRowFromFixture` (recorded JSON → row), `TestEmitEndToEnd`
   (AC-1), `TestPipeKillTimer` (AC-3). Fake `zellij` is a script the test
   writes to `t.TempDir()` — nothing binary committed.
3. **Fixture recording — declared machine step (the agent sandbox blocks
   agentsview):** CL, or any unsandboxed shell, runs
   `agentsview session get <id> --format json > grout/testdata/session-get.json`
   once, noting the binary version. Gate fixtures vendored from
   spacedock-subspace @9be5fbc with provenance noted in grout/README.md.
4. **Hermeticity:** `go test ./... && go vet ./...` in `grout/` pass with no
   network and no real agentsview/zellij needed.
5. **Live demo (sprint-0 exit, joint with slice (c)):** grout run in CL's
   fresh session → both rows rendered <10s (AC-4); probe result recorded
   (AC-5).

No live drill is needed beyond items 1 and 5; both piggyback on sessions
that exist anyway.

## Out of scope

- SSE `data_changed` consumption, many sessions, state mapping from
  agentsview signals, unbound handling (sprint 1).
- Glob-config gate discovery, folding (via the subspace binary — never
  reimplemented), pending/parked filtering (sprint 2).
- Any plugin change: agent-event parsing, rows rendering, click actions
  (slice c); the pipe-unblock fix (sibling slice).
- Verdict actions / `<log>.addr` (sprint 3 / M2).
- Config file, flags beyond the two positionals, daemonization.
- agentsview version management beyond recording which version produced the
  fixture.

## Stage Report: ideation

- DONE: ACs split offline vs interactive, with at least one value-measuring AC (e.g. both row kinds observable at the plugin within N seconds of emit)
  AC-1–3 offline, AC-4/AC-5 interactive; AC-1 (wire-level, fixture-value baseline) and AC-4 (plugin-observable within 10s, spike baseline 1–6s) measure the end value.
- DONE: Row protocol pinned exactly to the plan's two typed kinds on pipe name agent-event; payload-size-limit verification in the test plan
  "Row protocol (pinned)" section fixes field sets/types/clamp on pipe name `agent-event`; AC-2 enforces exact key sets; test plan item 1 is the size probe (4/64/256 KiB, receipt via debug trace or toggle fallback), listed first as the riskiest unproven mechanism.
- DONE: Go module layout, hardcoded-config shape, and fire-and-forget kill-timer semantics specified concretely
  `grout/` single-package layout with file-by-file roles; Config struct with literal values and two positional overrides; kill timer = exec.CommandContext + 5s timeout + WaitDelay, per-row independence, no `--plugin` ever, exit 0/1.

### Summary

Designed the sprint-0 grout skeleton: a one-shot Go binary (new `grout/`
module) that fetches one agentsview session, reads the vendored subspace
playground gate brief, and emits the plan's two pinned row kinds as JSON
lines on `agent-event` via per-row `zellij pipe` with a 5s kill timer.
Riskiest unproven mechanism named (pipe payload size ceiling — untested in
the spike) with the probe first in the test plan; transport, brief shapes,
and the session-get surface are spike/recon-proven. Surfaced two environment
facts: PATH agentsview is v0.32.1 vs the plan-pinned v0.36.1 (surface used
exists in both), and the safehouse sandbox blocks agents from running
agentsview — fixture recording is a declared CL step.

## Stage Report: implementation

- DONE: Red-first evidence recorded for the five planned tests (TestRowProtocol, TestGateRowFromBrief, TestSessionRowFromFixture, TestEmitEndToEnd, TestPipeKillTimer)
  Each red preceded its fix; passing count 0 → 6 (the clamp case lives in sibling TestRowProtocolSummaryClamp, matched by `-run TestRowProtocol`). Exact red outputs:
  - TestRowProtocol: build fail `rows_test.go:46: undefined: sessionInfo` (+ BuildSessionRow/gateInfo/BuildGateRow) → green f6ec63d
  - TestGateRowFromBrief: build fail `gate_test.go:15: undefined: briefPathForLog`, `undefined: GateFromLog` → green 2c1e5f2
  - TestSessionRowFromFixture: build fail `agentsview_test.go:18: undefined: decodeSession` → green efdc72e
  - TestEmitEndToEnd: build fail `emit_test.go:75: undefined: Config`, `undefined: run` → green 32df34e
  - TestPipeKillTimer: predicted hang — run wedged in cmd.Run's stderr pipe copy against the never-exiting fake, `panic: test timed out after 15s`, `FAIL zaphod/grout 15.284s` → green f74dd75 (exec.CommandContext under PipeTimeout + WaitDelay 2s), now 2.1s elapsed, both rows attempted, children reaped (kill -0 ESRCH)
- DONE: Hermetic suite green: go test ./... and go vet ./... in grout/ with no network, no real agentsview/zellij
  With GOPROXY=off GOFLAGS=-mod=readonly: `ok zaphod/grout`, 6/6 pass, vet clean. Fakes are test-written sh scripts in t.TempDir(); nothing binary committed. gofmt clean; `go build` links.
- DONE: Session fixture recorded binary-authentically and its provenance (version, recipe) noted in grout/README.md; AC-5 payload probe run or explicitly parked demo-ready with reason
  Recorded off /opt/homebrew/bin/agentsview v0.36.1 (commit 4c4bb56): AGENTSVIEW_DATA_DIR=<tmp> CLAUDE_PROJECTS_DIR=<tmp>, synthetic 2-line session JSONL, `session sync`, `session get <id> --format json`. Recipe + provenance in grout/README.md (2f34b07, 13c4d1f). AC-5 parked demo-ready below.

### AC-5 payload probe — parked demo-ready

Reason: interactive-only. Receipt confirmation needs a live zellij session running the rail (`debug "1"` trace in `$TMPDIR/zellij-<uid>/zellij-log/zellij.log`, or the visible-toggle fallback); the agent shell has no tty or session and cannot observe the rail. The entity already schedules the probe to piggyback on the AC-4 live demo. Ready commands (inside the session; guard each with `timeout 10` until the pipe-unblock slice lands):

    zellij pipe --name agent-event -- "$(python3 -c 'print("x"*4096)')"
    zellij pipe --name agent-event -- "$(python3 -c 'print("x"*65536)')"
    zellij pipe --name agent-event -- "$(python3 -c 'print("x"*262144)')"

Fallback without the debug key: the same sizes on pipe name `toggle` — the rail visibly toggling proves the payload traversed. Record the measured ceiling (or "≥256 KiB — unbounded for our purposes") in this entity; a ceiling below ~1 KiB invalidates the protocol.

### Summary

Built the sprint-0 grout skeleton on branch spacedock-ensign/grout-skeleton (7 commits, 2f34b07..13c4d1f), strict red-first TDD, one behavior per commit: row protocol pinned to plan decision 3; brief derivation + frontmatter parse against the vendored playground pair; session decode off the recorded v0.36.1 fixture (termination_status → state, first_message → summary); end-to-end two-row emit on agent-event with no --plugin ever; and the 5s kill timer proving a wedged pipe cannot wedge grout. Environment notes: fixture recording auto-spawned an agentsview daemon (pid 29962) — stopped via `serve stop`, port 8080 verified free; ~/.agentsview untouched. Provenance nuance recorded in grout/README.md: the playground gate pair is a working-copy artifact of spacedock-subspace @ 9be5fbc (gitignored there as /playground*, not in that commit's tree); its shapes match the brief code at 9be5fbc.

## Refutation audit — validation, throwaway checkout @13c4d1f

Throwaway: `git archive 13c4d1f` extracted to the session scratchpad (never
the implementation worktree). Baseline there first: `go test ./...` +
`go vet ./...` green under GOPROXY=off. Probes live in a throwaway
`refute_test.go` (6 tests) plus two fixture mutations; suite re-verified
green after every restore. No REFUTED findings; two caveats noted below.

1. **Protocol drift vs plan decision 3.** Key sets transcribed independently
   from `docs/plan-agent-rail.md` decision 3 match rows.go struct tags and
   the test baselines (session 7 keys, gate 9). Probe: all-empty source
   fields → exact key sets survive (no omitempty-style drop); zero `round`
   still a JSON number; `ts` from a nanosecond instant still parses RFC3339.
   Probe: summary carrying raw `\n`/`\r`/quotes → marshaled payload has no
   raw newline bytes, one-line-per-invocation holds. SURVIVED.
2. **Kill-timer child reaping under variants.** Variant: TERM-immune child
   (`trap '' TERM INT` + loop) — both recorded pids reaped (kill -0 →
   ESRCH), elapsed 2.14s < the 4s wall, error surfaced; CommandContext's
   SIGKILL is untrappable. Variant: child exits 0 leaving a `sleep 300 &`
   grandchild holding inherited pipes — production shape (stderr is an
   *os.File, as main.go passes os.Stderr): run returns in 30ms, exit 0;
   test shape (bytes.Buffer stderr): bounded at 4.3s by WaitDelay (2s/row)
   and surfaces the error. Grout never wedges. SURVIVED, with caveat:
   grandchildren themselves survive (4/4 alive after runs) — the kill scope
   is the direct child only; real `zellij pipe` spawns no grandchildren,
   and boundedness holds regardless.
3. **Fixture-provenance holes.** Mutation: session-get.json
   `termination_status` awaiting_user→running → TestSessionRowFromFixture +
   TestEmitEndToEnd fail on exactly that field (restored). Mutation:
   playground-gate.md `round: 1`→`2` → TestGateRowFromBrief +
   TestEmitEndToEnd fail (restored). Baselines demonstrably flow from the
   fixtures, not from hardcoded twins. Source check: spacedock-subspace is
   at HEAD 9be5fbc as the README claims; `DecisionLogPath`
   (internal/brief/brief.go) matches grout's `briefPathForLog` inversion;
   the vendored playground pair is byte-identical to the working copy
   (diff clean); the README's gitignored-working-copy nuance is accurate.
   SURVIVED, with caveat: subspace special-cases `brief.md` →
   `decision-log.jsonl`, which the TrimSuffix inversion does not cover
   (would derive `decision-log.jsonl.md`). The entity pins only the
   gate-foo shape and defers discovery to sprint 2 — flag for the sprint-2
   glob config.
4. **Clamp edge cases.** Probes: 510 ASCII + 4-byte emoji straddling 512 →
   510 bytes, valid UTF-8, prefix of source; exactly 512 → unchanged;
   513 → 512; empty → empty; clamp below the first rune (`"世"`, 2) → ""
   without panic. Probe: 600×0xff direct call → no panic, returns 512
   still-invalid bytes (0xff looks like a rune start); ingress probe shows
   `decodeSession` sanitizes invalid UTF-8 to U+FFFD on decode, so
   clampSummary never sees invalid input via the real path. SURVIVED.

## Demo script — sprint-0 exit gate (AC-4 + AC-5), CL drives

Setup (once):

1. Build + install the plugin from the branch with the pipe-unblock fix
   landed: `./build.sh && ./install.sh` at the zaphod repo root (installs
   `~/.config/zellij/layouts/zaphod.kdl`).
2. Enable the trace observation path (pre-slice-(c) there is no rendered
   row): add `debug "1"` beside each `rail "1"` in the installed
   `~/.config/zellij/layouts/zaphod.kdl` plugin blocks.
3. Start a fresh zellij session on that layout. In any shell (log verified
   present at this path on this machine):

       LOG="${TMPDIR%/}/zellij-$(id -u)/zellij-log/zellij.log"
       tail -f "$LOG" | grep --line-buffered 'pipe recv name=agent-event'

Spot-check (seconds, before any real time is spent — proves the drill
infra end to end):

4. Inside the session: `timeout 10 zellij pipe --name agent-event -- ping; echo exit=$?`
   Expect: prompt returns quickly, `exit=0`, and one
   `zaphod-trace[N]: pipe recv name=agent-event` line in the tail.
   No trace line → the debug key is not active in the running layout (fix
   step 2, restart) or fall back to `--name toggle` (rail visibly toggles).
   Hang until timeout → the running plugin predates the pipe-unblock fix.

AC-4 — both row kinds at the plugin within 10s:

5. Pick a live session id: `agentsview session list | head`.
6. Inside the session, from the grout worktree root
   (`/Users/clkao/git/zaphod/.worktrees/spacedock-ensign-grout-skeleton`):

       time go run ./grout <session-id>

   (gate log defaults to the vendored
   `grout/testdata/playground-gate.decisions.jsonl`; pass a real gate log
   as argv[2] to demo against the live playground copy instead.)
7. Expect: exit 0, stderr empty, wall time well under 10s (spike baseline
   1–6s), and TWO new `pipe recv name=agent-event` trace lines — session
   row then gate row. That is AC-4's pre-slice-(c) observable; once slice
   (c) lands, the rendered rail rows replace the trace as the observation.
   Failure shape: `pipe timeout after 5s: kind=…` on stderr + exit 1 means
   the pipe path is wedged; grout still exits within ~12s by design.

AC-5 — payload ceiling probe (parked from implementation; run while the
session is up, one size at a time so receipt is attributable):

8.     timeout 10 zellij pipe --name agent-event -- "$(python3 -c 'print("x"*4096)')"    # 4 KiB
       timeout 10 zellij pipe --name agent-event -- "$(python3 -c 'print("x"*65536)')"   # 64 KiB
       timeout 10 zellij pipe --name agent-event -- "$(python3 -c 'print("x"*262144)')"  # 256 KiB

   One new trace line after each command = receipt at that size. The trace
   logs the name only, not payload length — hence one-at-a-time. Payload
   integrity (untruncated content) becomes checkable when slice (c) parses
   payloads. Fallback without the debug key: the same sizes on
   `--name toggle`; the rail visibly toggling proves traversal.
9. Record the result in this entity: the measured ceiling, or "≥256 KiB —
   unbounded for our purposes". A ceiling below ~1 KiB invalidates the
   protocol → back to ideation.

## Stage Report: validation

- DONE: Every offline AC (AC-1, AC-2, AC-3) re-verified by re-running its Verified-by command yourself in the worktree — verdicts from re-execution with your own outputs, never the implementer's numbers
  Worktree clean at 13c4d1f (identity = implementation's final commit). AC-1 `go test -run TestEmitEndToEnd`: PASS 0.44s. AC-2 `-run TestRowProtocol`: 2/2 PASS (protocol + clamp sibling). AC-3 `-run TestPipeKillTimer`: PASS, 2.09s elapsed, inside the 2×PipeTimeout+2s wall. Full suite + `go vet` + `gofmt -l` clean under GOPROXY=off.
- DONE: Refutation audit on a THROWAWAY checkout (never the implementation worktree) with named attacks: protocol drift vs plan decision 3, kill-timer child reaping under variants, fixture-provenance holes, clamp edge cases — each attack's probe and outcome recorded
  git-archive of 13c4d1f in the session scratchpad; 6 throwaway probe tests + 2 fixture mutations, outcomes in "Refutation audit" section above. No REFUTED; two caveats: grandchild kill scope (bounded regardless; production stderr path unaffected), brief.md derivation special case (sprint-2 discovery concern).
- DONE: Demo script prepared for CL's live gate: exact setup + commands + expected observations for AC-4 (both rows at plugin <10s) and the parked AC-5 probe (4/64/256 KiB), including the pre-slice-(c) debug-trace observation path and a cheap spot-check that the drill infra works before CL's time is spent
  "Demo script" section above: install + `debug "1"` setup, seconds-cost `zellij pipe … -- ping` spot-check with trace grep, AC-4 run + expected observations and failure shape, AC-5 one-size-at-a-time probes with trace receipt and toggle fallback. Log path and every command verified present/runnable on this machine.

### Summary

Independently re-verified all three offline ACs by re-execution in the
worktree at 13c4d1f — all pass with my own outputs. Ran a four-attack
refutation audit on a throwaway git-archive checkout: no REFUTED findings;
the protocol, kill timer, fixtures, and clamp survive, with two recorded
caveats (grandchild processes outlive the kill scope though grout stays
bounded; the brief.md→decision-log.jsonl derivation special case is
uncovered until sprint 2's discovery). Fixture mutations proved AC-1's
baselines flow from the fixtures. Prepared CL's live-gate demo script for
AC-4/AC-5 with a seconds-cost spot-check ahead of the expensive run;
interactive ACs remain CL's to settle at the demo.
