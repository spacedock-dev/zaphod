---
commissioned-by: spacedock@0.24.0-pre2
entity-type: task
entity-label: task
entity-label-plural: tasks
id-style: sd-b32
state: .spacedock-state
stages:
  defaults:
    worktree: false
    concurrency: 2
  states:
    - name: backlog
      initial: true
    - name: ideation
      gate: true
      concurrency: 7
    - name: implementation
      worktree: true
    - name: validation
      worktree: true
      fresh: true
      feedback-to: implementation
      gate: true
    - name: done
      terminal: true
---

# Agent-rail development

Tracks Zaphod delivery work. The authoritative sequence is
`docs/roadmap.md`; `docs/zaphod-workspace-architecture.md` defines the
long-term boundary; and
`docs/archive/plan-agent-rail-prototype-2026-07-07.md` records historical
prototype work. The current per-tab rail is a product baseline, not disposable
scaffolding.

Each task must advance one operator outcome or close a measured failure in an
existing outcome. The workflow remains its own dogfood tenant: its validation
reviews leave decision logs at the globbable gates location, and the rail may
surface those reviews. The rail opens the provider's UI; it never writes an
inline verdict.

Sprint membership is frontmatter, not a hard-coded roadmap list. Query a
sprint with `spacedock status --workflow-dir docs/agent-rail-dev --where
sprint=<slug>`; add `--where 'sprint-readiness != defer' --fields group` to
see its ready delivery roles. Readiness informs First Officer policy; it does
not override the workflow's normal dispatch guards.

## File Naming

Each task lives as either:

- a flat markdown file `{slug}.md` (default), or
- a folder `{slug}/` containing `index.md` as the canonical entity file, when
  the task produces per-stage artifacts (transcripts, drill evidence, design
  notes) that belong alongside the tracker.

Slugs are lowercase, hyphens, no spaces. Example: `plugin-pipe-unblock.md`.

## Schema

Every task file has YAML frontmatter. Fields are documented below; see
**Task Template** for a copy-paste starter.

### Field Reference

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | Unique identifier, SD-B32 (see ID Style) |
| `title` | string | Human-readable task name |
| `status` | enum | One of: backlog, ideation, implementation, validation, done |
| `source` | string | Where this task came from (plan sprint, retrospective, finding) |
| `sprint` | string | Sprint slug used for membership queries, for example `s1-trusted-test-profile-onramp` |
| `group` | string | Role within that sprint, such as `walking-skeleton` or `contingent-enablement` |
| `sprint-readiness` | string | `ready` or `defer`; a delivery-planning filter, not a machine dispatch lock |
| `started` | ISO 8601 | When active work began |
| `completed` | ISO 8601 | When the task reached terminal status |
| `verdict` | enum | PASSED or REJECTED — set at validation |
| `score` | number | Priority score, 0.0–1.0 (optional) |
| `worktree` | string | Worktree path while a dispatched agent is active; sticky across non-terminal advancements, cleared at terminal merge |
| `issue` | string | GitHub issue reference (optional cross-reference) |
| `pr` | string | Unused — this workflow ships by direct local merge, no PR ritual |
| `mod-block` | string | Pending mod-declared blocking action, format `{lifecycle_point}:{mod_name}` |

### ID Style

The `id-style` is `sd-b32`: `id` stores the full stable 24-character lowercase
SD-B32 stored ID (SHA-256 digest material formatted with Spacedock's
human-safe alphabet `0123456789abcdefghjkmnpqrstvwxyz`), minted at creation
via `spacedock new <slug> --id-seed "<slug>"`. Status tables display the
shortest unique prefix (`MIN_PREFIX: 2`). Generated IDs make concurrent
creation across worktree branches safe — no shared counter. Use
`status --validate` before trusting workflow state and `status --resolve
<ref>` to resolve slugs, stored IDs, or address prefixes.

```yaml
id-style: sd-b32
```

## Stages

### `backlog`

A task enters backlog as a seed from the sprint plan (or a finding promoted
from triage). Ungated: the FO advances a ready task only when its sprint-entry
gate permits it; CL reprioritizes conversationally.

- **Inputs:** `docs/roadmap.md` sprint outcome and ordering; the task's seed
  description; and, when relevant, the current per-tab baseline.
- **Outputs:** a one-paragraph problem statement and the sprint it serves.
- **Good:** the task is the smallest unit that demos an operator outcome or
  closes a measured baseline failure; the roadmap's current outcome leads.
- **Bad:** a task that bundles two behaviors; a task whose exit can't be demoed in the fresh zellij session.

### `ideation`

CL greenlit the task; a worker designs it: problem, approach, acceptance
criteria as entity-level end-state properties with `Verified by:` clauses, and
a test plan matching the AC's level of abstraction.

- **Inputs:** `docs/roadmap.md` for the sprint outcome and explicit deferrals;
  `docs/zaphod-workspace-architecture.md` for durable product boundaries;
  the shipped baseline in `README.md`; `SPEC.md` landmines;
  `docs/docking-approach.md`; and relevant prototype records. Historical
  `docs/archive/plan-agent-rail-prototype-2026-07-07.md` decisions are
  evidence, not binding product architecture.
- **Outputs:** entity body filled: Problem / Proposed approach / Acceptance criteria with `Verified by:` clauses / Test plan / Out of scope; ACs split into **offline** (agent-reproducible) and **interactive** (settled only by CL's live demo); the task's riskiest unproven mechanism named, with the smallest end-to-end check that would invalidate the design listed first in the test plan — or the auditable negative `no spike needed: {the proven mechanisms it relies on}` on the record; when the task changes user-visible behavior (keybinds, rows, layout), a concrete doc diff proposed in the body and reviewed at this gate.
- **Good:** at least one AC measures the end value the task exists for, against an independent baseline that can move the wrong way (a count, a timing, a behavior, resulting on-disk state) — a mechanism-only AC counts only when paired with the value-measuring AC it serves; every AC's expected value comes from outside the file under test; fixtures specified in zellij's real single-line dump shape where dumps are involved; the design names which existing pure functions it extends.
- **Bad:** an AC provable only by reviewing the entity's own prose; a string/substring/regex match over a file the implementer also writes (a tautology — the check polices its own author); a design that reaches beyond the task's sprint exit criterion; inventing new mechanisms when the spike already proved one.

### `implementation`

The design is approved and the deliverable is built in a dedicated worktree —
strict TDD, one behavior per commit.

- **Inputs:** the approved ideation body; the repo at the worktree branch.
- **Outputs:** commits satisfying the AC (each: red test first, red output recorded in the stage report with the failure reason, minimal fix, suite green); a stage report with before/after test counts and the exact red output; for plugin work `cargo test` + `cargo check --tests` are the native verification (`./build.sh` only when a demo needs the wasm; native `cargo build` link-fails by design); for grout work `go test ./...` + `go vet`; passing exact-head Roborev `code_completion` evidence, including the synthesis parent job ID, exact reviewed range and head, panel name, required-member execution outcomes, parent verdict, and finding dispositions.
- **Good:** the red test fails for the predicted reason before the fix; the smallest reasonable diff; surrounding style matched; new dump fixtures use zellij's real single-line shape.
- **Bad:** fix-first-test-later; unrelated reformatting (the repo carries pre-existing fmt violations — leave them); skipping or evading a pre-commit hook; launching `code_completion` while the exact-tip `quick` review is pending or has an unresolved Medium-or-higher finding; treating `quick` as the implementation-exit verdict; using `roborev fix`, `roborev refine`, or the Roborev agent hook; bundling two behaviors into one commit; committing without the stage report's red/green evidence; a "one more polish" commit after the task has been handed to validation — confirm no pending round-trip before advancing.

After the final candidate commit, implementation MUST run `roborev wait HEAD`
for that commit's existing post-commit `quick` review. A Medium-or-higher
finding blocks the expensive panel: disposition it in implementation, fix and
commit when warranted, then wait on the new exact HEAD. Low findings remain
advisory and do not force another round. Only after the exact-tip `quick`
review clears that cost gate may implementation run the required panel:

```bash
roborev review --repo <canonical-project-root> \
  --branch=<implementation-branch> --base main \
  --panel code_completion --min-severity medium --wait
```

Preserve the synthesis parent job ID even when `--wait` exits nonzero, then
inspect it with `roborev show --job <parent-id> --json`. Implementation exits
only when the stored range is `merge-base(main, head)..head`, the reviewed head
is the current branch tip, every configured required member appears exactly
once without an execution failure, and the `code_completion` synthesis parent
verdict is PASS. The synthesis parent is authoritative; `quick` is only the
scheduling and cost gate. Every synthesized finding receives a `fix`, `rebut`
with repository evidence and replacement-panel adjudication, or `needs
decision` disposition. Any code-changing commit invalidates the old panel and
requires a passing exact-head replacement before validation.

### `validation`

The demo is the gate. A fresh agent — no shared context with the implementer
— reproduces the **offline** ACs by re-running each `Verified by:` clause,
runs an adversarial refutation audit on a throwaway checkout, and prepares
the **demo script** for the interactive ACs: exact commands, setup state, and
what CL should see at each step. CL drives the demo live in the fresh zellij
session — for interactive ACs, CL is the validator; a fresh agent cannot
reproduce a live floating-TUI drill. The gate presents artifact + demo result
through spacedock-subspace: a brief plus `<artifact>.decisions.jsonl` under
the globbable gates location (`docs/agent-rail-dev/.spacedock-state/gates/`),
reviewed via `subspace-tui` (or the browser gate server). Either
gate-approval to done or rejection back to implementation with concrete
findings.

- **Inputs:** the worktree at the implementation's final commit, identity-checked by raw commit SHA (a sibling may still be mutating the worktree); the ideation AC split; the stage report's claims; the recorded Roborev `code_completion` synthesis parent.
- **Outputs:** independent verification that the recorded synthesis parent covers the frozen `merge-base(main, head)..head` range and current head, names the `code_completion` panel, contains every required member exactly once without execution failure, and has a PASS parent verdict; per-offline-AC verdicts with independently reproduced evidence (re-run commands, not re-read reports); a refutation audit on a throwaway checkout — never the implementation worktree — naming the concrete attack scenarios attempted (false positives/negatives, panic/indexing paths, caller impact, semantic drift vs. pre-diff behavior) and why each failed, or a REFUTED with file:line; the demo script; the subspace review record with the demo outcome.
- **Good:** a cheap fixture or single-command spot-check proves the drill infrastructure works end-to-end before CL's time is spent on the expensive live run; verdicts derived from re-execution; an attack survived is documented with the exact probe; "the finding's premise is false" is a valid and valuable outcome — stop and report rather than validating a fix against a false premise.
- **Bad:** trusting the implementer's numbers; rerunning an unchanged passing `code_completion` panel instead of verifying its stored evidence; accepting a stale panel after any fixing commit; treating missing quick-review coverage as a clean review; a SURVIVES with no named attacks; a fresh agent "reproducing" an interactive AC it cannot actually drive; validating the letter of an AC whose served end value regressed; rubber-stamping a stale comment or doc claim the diff made false.

Validation fetches the recorded parent with
`roborev show --job <parent-id> --json` and verifies the frozen evidence; it
does not rerun an unchanged passing panel. Validation remains behavior-focused
and independently reproduces every offline AC. If validation finds a code
defect, it routes the task back to implementation. Any fixing commit
invalidates the prior panel and requires a passing replacement before
validation resumes.

### `done`

Terminal: the task's worktree branch is merged directly into the working
branch by the merge ceremony (no PR — this repo has no remote by choice),
`completed` set, `verdict: PASSED`, entity archived. Reached via real merge,
not a manual flag flip. A task whose only output is a decision with nothing
shipped does not terminalize as PASSED — the decision is recorded in the
sprint plan instead.

## Workflow-specific rules

- **No PR machinery.** Shipping is a direct local merge at terminal; the `pr`
  field stays empty. If the repo later gains a remote and a PR ritual, refit
  with the pr-merge mod rather than adding stages.
- **Subspace review channel.** Validation-gate artifacts are presented as
  subspace reviews with decision logs under
  `docs/agent-rail-dev/.spacedock-state/gates/` — globbable at
  `docs/agent-rail-dev/.spacedock-state/gates/*.decisions.jsonl`, versioned on
  the state branch, zero code-branch churn. Manual presentation (CL floats
  `subspace-tui` in the fresh session) until the rail's sprint-2 task
  automates discovery. The FO still owns the gate; subspace is the
  presentation and record surface.
- **Test-first authoring, external-proof ACs, and detached adversarial audit**
  (the dev-shape proof disciplines) are mandatory here, folded into the
  implementation and validation stage definitions above.
- **Dogfood posture.** The historical sprint-0 pipe-unblock work established
  the rail's visible-not-blocking safety baseline. Keep that behavior in every
  current task. `install.sh` points the global layout only at the primary
  checkout artifact. Unmerged candidate live validation uses
  `./tests/zellij-tmux-smoke-test.sh` from the candidate worktree; it drives
  Zellij through a disposable tmux server and isolated config/data/socket
  roots, never standing global config or layout files.
- **Park-for-demo is correct posture.** When a task's next step is CL's live
  demo, parking it demo-ready and waiting for CL's window is right — not a
  stall. The FO keeps other tasks moving meanwhile.
- **Live e2e before merge for output-shape changes.** A change to the
  plugin's pipe payloads, row/pane output shape, or launch wiring must be
  driven in a real zellij session before merge — offline tests prove the
  logic, never the surface.
- **Spike discipline binds infra too.** Build/install/rollout plumbing
  changes (install.sh, layout wiring, grout deployment) get the same
  smallest-end-to-end exercise first as feature tasks.
- **Approval is explicit.** A live grant, a demo pass, or a merge go-ahead is
  never inferred from silence, from acknowledgment of a summary, or from a
  prior gate approval — only an explicit yes counts.

## Workflow State

View the workflow overview:

```bash
spacedock status --workflow-dir docs/agent-rail-dev
```

Find dispatchable tasks ready for their next stage:

```bash
spacedock status --workflow-dir docs/agent-rail-dev --next
```

## Task Template

```yaml
---
id:
title: Task title here
status: backlog
source:
sprint:
group:
sprint-readiness:
started:
completed:
verdict:
score:
worktree:
issue:
pr:
mod-block:
---

## Problem

What is broken or missing, and why it matters now.

## Proposed approach

How the implementation will address the problem. Concrete enough that a worker can start.

## Acceptance criteria

Each AC names a property of the finished task (not a stage action) and how it is verified.

**AC-1 — End-state property.**
Verified by: grep / test name / file path / command a future reader can reproduce.

## Test plan

What tests verify the implementation, estimated cost, whether a live drill is needed.

## Out of scope

What this task deliberately does not address.
```

## Commit Discipline

- Commit status changes at dispatch and merge boundaries
- Commit task body updates when substantive
- Implementation commits land on the worktree branch; merge to the working
  branch happens directly at terminal (no PR)
