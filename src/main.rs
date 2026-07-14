// ABOUTME: Clickable pane-switcher sidebar for zellij — lists panes in its own tab with their
// ABOUTME: last terminal line; Alt-/ flips it between a docked rail and a 1-col sliver.

use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use zellij_tile::prelude::*;

mod agent;

// Emits one greppable line to the plugin log when the "debug" config key is
// set. Plugin stderr is routed to zellij.log by the server's LoggingPipe
// (zellij-server plugin_loader sets stderr to a LoggingPipe; stdout is the
// pane render, so println! must never carry trace output). Off, it is a
// bare bool check: the arguments are not evaluated.
macro_rules! trace {
    ($self:expr, $($arg:tt)*) => {
        if $self.debug {
            eprintln!("zaphod-trace[{}]: {}", $self.plugin_id, format_args!($($arg)*));
        }
    };
}
const STATUS_POLL_SECS: f64 = 2.0;
// A pane-status call that stalls this long is wedge-classified and aborts
// the rest of the poll pass. Healthy calls return well inside the 2s poll
// timer; the observed wild GetPaneRunningCommand wedge is ~13s per call —
// 1s sits well clear of both.
const WEDGE_THRESHOLD: Duration = Duration::from_secs(1);
// Sidebar widths in the two swap-layout states (mirrors layouts/zaphod.kdl).
const DOCKED_COLS: usize = 28;
const UNDOCKED_COLS: usize = 1;
// Below this render width the rail is the undocked sliver (docked is 28,
// undocked is 1) — too narrow for status text, so it need not poll.
const STATUS_MIN_COLS: usize = 8;
// How long after a deferred steer fires that repeat presses for its tab are
// still swallowed: the steered collapse becomes visible a beat after the
// steer itself, and a press inside that gap would instantly undo the toggle
// the user is still watching land.
const TOGGLE_COOLDOWN: Duration = Duration::from_millis(600);

fn permissions_for_config(config: &BTreeMap<String, String>) -> Vec<PermissionType> {
    let mut permissions = vec![
        PermissionType::ReadApplicationState,
        PermissionType::ChangeApplicationState,
        PermissionType::ReadPaneContents,
    ];
    if config
        .get("recipient_token")
        .is_some_and(|token| !token.is_empty())
    {
        permissions.push(PermissionType::ReadCliPipes);
    }
    permissions.extend([PermissionType::Reconfigure, PermissionType::RunCommands]);
    permissions
}

#[derive(Default)]
struct Sidebar {
    rows: Vec<Row>,
    plugin_id: u32,
    rendered_once: bool,
    permissions_requested: bool,
    permissions_granted: bool,
    own_tab: Option<usize>,
    own_url: Option<String>,
    config: BTreeMap<String, String>,
    debug: bool,
    nav_mode: bool,
    nav_selected: usize,
    return_focus: Option<u32>,
    active_tab: Option<usize>,
    // The active tab as last reported by TabUpdate (server-authoritative
    // position + active flag). Unlike active_tab — which perform_toggle
    // overwrites with the get_focused_pane_info tab_id→position translation
    // that diverges across instances — this is never touched by the toggle
    // path, so it is the reliable signal the status-poll visibility gate uses
    // to decide "is my tab the one on screen".
    reported_active_tab: Option<usize>,
    // Per-tab swap-layout state keyed by display position. Caveat: zellij
    // reports (None, false) for tabs with at most one selectable tiled pane,
    // so swap damage there is invisible.
    tab_states: BTreeMap<usize, TabState>,
    own_floating: bool,
    instances: Vec<SidebarInstance>,
    pending_steer: Option<PendingSteer>,
    close_requested: bool,
    // The tab whose deferred steer last fired, and when: repeat presses for
    // it inside TOGGLE_COOLDOWN are swallowed as bounce.
    toggle_cooldown: Option<(usize, Instant)>,
    // We requested this client's temporary Alt / route while the rail was
    // visible. Reconfigure has no acknowledgement, so this only prevents
    // duplicate requests; it never authorizes a later pipe.
    toggle_route_requested: bool,
    // The last render width, used to gate status polling: an undocked sliver
    // has no room to show status, so it skips the poll.
    last_cols: usize,
    // Per-pane status-poll backoff keyed by terminal pane id: a pane whose
    // get_pane_running_command keeps timing out is polled exponentially less
    // often instead of every timer. Pruned to the current rows each poll.
    poll_backoff: BTreeMap<u32, PollBackoff>,
    // Wedge drill knob (see wedge_poll_secs): seconds each status poll sleeps
    // in place of its get_pane_running_command call. None outside drills.
    wedge_poll_secs: Option<u64>,
    // Agent sessions and pending gates fed over the agent-event pipe,
    // rendered as the AGENTS/GATES sections below the pane rows. Upserted in
    // arrival order, never expired (grout is one-shot in sprint 0).
    sessions: Vec<SessionEvent>,
    gates: Vec<GateEvent>,
    // A named pipe reaches every running copy of the plugin. Agent rows are
    // therefore allowed through only after this rail's current PaneUpdate
    // has been matched by a later, unambiguous TabUpdate. The server tab id
    // is the pipe recipient; display position is only the bridge from the
    // PaneManifest to that stable id.
    agent_manifest_generation: u64,
    agent_manifest_seen: bool,
    agent_recipient: Option<AgentRecipient>,
    // Each terminal pane's cwd as last polled via get_pane_cwd — the data
    // session binding matches against. Keyed by pane id, so it survives the
    // manifest's row rebuilds; pruned to the current rows each poll pass. A
    // failed poll keeps the previous entry (stale-not-blank).
    pane_cwds: BTreeMap<u32, PathBuf>,
}

// One pane's status-poll backoff. get_pane_running_command's timeout Err is
// the same shape as not-found, so a naive poll retries a stuck pane every
// timer forever, each retry blocking the plugin thread on a ps fork. After a
// failed poll the pane is skipped for a growing number of timers; a success
// resets it.
#[derive(Default, Clone, Copy)]
struct PollBackoff {
    failures: u32,
    skip: u32,
}

impl PollBackoff {
    // Whether this pane is due for a poll this timer; consumes one skip if
    // it is backed off.
    fn due(&mut self) -> bool {
        if self.skip > 0 {
            self.skip -= 1;
            false
        } else {
            true
        }
    }

    // Records a poll's outcome: a failure grows the backoff, a success clears
    // it.
    fn record(&mut self, failed: bool) {
        if failed {
            self.failures += 1;
            self.skip = backoff_skips(self.failures);
        } else {
            self.failures = 0;
            self.skip = 0;
        }
    }
}

// The dock state a swap-set override is driving its tab toward, deferred
// until TabUpdate reports the overridden set installed: the server
// dispatches the override on its own thread, so a step fired immediately
// after could still cycle the old swap set. The direction of the step is
// decided at fire time from the entry the override actually landed on.
#[derive(Debug, Clone, Copy, PartialEq)]
struct PendingSteer {
    tab: usize,
    target: DockState,
}

// One tab's swap-layout state from TabUpdate. `id` is the server's stable
// tab id — the key dump_session_layout_for_tab and get_focused_pane_info
// speak — which diverges from the display position once any tab is closed
// or reordered.
#[derive(Debug, Clone, PartialEq, Default)]
struct TabState {
    id: usize,
    swap_name: Option<String>,
    swap_dirty: bool,
    // While a tab's floating panes are visible, swap_name speaks for the
    // FLOATING layer (tab/mod.rs swap_layout_info) — every tab's floating
    // list carries a birth "BASE" — so the tiled layer's state is unreadable.
    floating_visible: bool,
}

// A tab recipient proved from one PaneUpdate generation and a later complete
// TabUpdate snapshot. `stable_tab_id` deliberately accepts zero: Zellij uses
// zero for the first server tab, so Option—not a numeric sentinel—represents
// an unavailable mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AgentRecipient {
    manifest_generation: u64,
    own_position: usize,
    stable_tab_id: usize,
}

// One sidebar plugin pane somewhere in the session, as seen in the manifest.
#[derive(Debug, Clone, Copy, PartialEq)]
struct SidebarInstance {
    pane_id: u32,
    tab: usize,
    floating: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
struct Row {
    pane_id: u32,
    title: String,
    focused: bool,
    agent: agent::AgentFields,
}

// One agent-event payload: the two row kinds pinned by plan decision 3
// (docs/plan-agent-rail.md). Only the fields the rail acts on are declared;
// everything else (ts, workflow, entity, future additions) is tolerated and
// ignored, and a declared field that is absent defaults — a session without
// cwd simply renders unbound.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
enum AgentEvent {
    Session(SessionEvent),
    Gate(GateEvent),
}

#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
struct SessionEvent {
    #[serde(default)]
    id: String,
    #[serde(default)]
    pane_id: Option<u32>,
    #[serde(default)]
    cwd: String,
    #[serde(default)]
    agent: String,
    #[serde(default)]
    state: String,
    #[serde(default)]
    summary: String,
}

#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
struct GateEvent {
    #[serde(default)]
    log_path: String,
    #[serde(default)]
    entity_title: String,
    #[serde(default)]
    stage: String,
    #[serde(default)]
    round: u32,
    #[serde(default)]
    recommendation: String,
}

// Parses one agent-event payload line. Malformed JSON and unknown kinds are
// in-band errors; the caller drops the event with the reason traced.
fn parse_agent_event(payload: &str) -> Result<AgentEvent, String> {
    serde_json::from_str(payload).map_err(|error| error.to_string())
}

fn apply_agent_snapshot(
    sessions: &mut Vec<SessionEvent>,
    gates: &mut Vec<GateEvent>,
    payload: Option<&str>,
) -> Result<bool, String> {
    let payload = payload.ok_or_else(|| "missing snapshot payload".to_owned())?;
    let events: Vec<AgentEvent> =
        serde_json::from_str(payload).map_err(|error| error.to_string())?;
    let mut next_sessions = Vec::new();
    let mut next_gates = Vec::new();
    let mut ids = BTreeSet::new();
    let mut pane_ids = BTreeSet::new();
    for event in events {
        match event {
            AgentEvent::Session(session) => {
                let pane_id = session.pane_id.ok_or_else(|| {
                    format!("session {:?} has no registered pane_id", session.id)
                })?;
                if session.id.is_empty() || !ids.insert(session.id.clone()) {
                    return Err(format!("missing or duplicate session id {:?}", session.id));
                }
                if !pane_ids.insert(pane_id) {
                    return Err(format!("duplicate registered pane_id {pane_id}"));
                }
                next_sessions.push(session);
            }
            AgentEvent::Gate(gate) => next_gates.push(gate),
        }
    }
    let mut changed = *sessions != next_sessions;
    *sessions = next_sessions;
    for gate in next_gates {
        changed = apply_agent_event(sessions, gates, AgentEvent::Gate(gate)) || changed;
    }
    Ok(changed)
}

// Upserts one event into the rail's session/gate lists: sessions keyed by
// id, gates by log_path, insertion order kept, no expiry (grout is one-shot
// in sprint 0; lifecycle is sprint 1+). Returns whether stored state
// changed, so the pipe handler re-renders only on real updates.
fn apply_agent_event(
    sessions: &mut Vec<SessionEvent>,
    gates: &mut Vec<GateEvent>,
    event: AgentEvent,
) -> bool {
    match event {
        AgentEvent::Session(session) => {
            let key = session.id.clone();
            upsert(sessions, |existing| existing.id == key, session)
        }
        AgentEvent::Gate(gate) => {
            let key = gate.log_path.clone();
            upsert(gates, |existing| existing.log_path == key, gate)
        }
    }
}

fn upsert<T: PartialEq>(list: &mut Vec<T>, keyed: impl Fn(&T) -> bool, item: T) -> bool {
    match list.iter_mut().find(|existing| keyed(existing)) {
        Some(existing) if *existing == item => false,
        Some(existing) => {
            *existing = item;
            true
        }
        None => {
            list.push(item);
            true
        }
    }
}

// Registration supplies the only binding authority. The row remains bound
// only while that exact terminal id is present in this rail's current tab
// manifest. CWD, title, prompt, time, and row order never enter the join.
fn registered_session_pane(session: &SessionEvent, rows: &[Row]) -> Option<u32> {
    let pane_id = session.pane_id?;
    rows.iter()
        .any(|row| row.pane_id == pane_id)
        .then_some(pane_id)
}

// The gate's reviewable artifact, inverted from its decision-log path the
// same way grout derives the log from the brief: trim .decisions.jsonl, add
// .md. A log path without that suffix yields no brief — never float a wrong
// file.
fn brief_path_for_log(log_path: &str) -> Option<String> {
    log_path
        .strip_suffix(".decisions.jsonl")
        .filter(|stem| !stem.is_empty())
        .map(|stem| format!("{stem}.md"))
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum LineTarget {
    Header,
    Row(usize),
    SessionRow(usize),
    GateRow(usize),
    None,
}

// The rail's 1-based line map: pane rows first, then an AGENTS and a GATES
// section, each a header line plus two-line rows, present only when it has
// rows — an empty section occupies no lines at all. render prints in this
// exact order, so deriving click targets from the same counts keeps the
// click math aligned with the pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
struct SectionLayout {
    pane_count: usize,
    session_count: usize,
    gate_count: usize,
}

fn section_layout(pane_count: usize, session_count: usize, gate_count: usize) -> SectionLayout {
    SectionLayout {
        pane_count,
        session_count,
        gate_count,
    }
}

impl SectionLayout {
    // 1-based line of the AGENTS header, when the section renders.
    fn agents_header(&self) -> Option<usize> {
        (self.session_count > 0).then(|| 2 + 2 * self.pane_count)
    }

    // 1-based line of the GATES header, when the section renders.
    fn gates_header(&self) -> Option<usize> {
        (self.gate_count > 0).then(|| {
            let sessions = if self.session_count > 0 {
                1 + 2 * self.session_count
            } else {
                0
            };
            2 + 2 * self.pane_count + sessions
        })
    }

    fn target(&self, line: isize) -> LineTarget {
        if line <= 0 {
            return LineTarget::None;
        }
        // The pane region is the shipped pane-rows map, byte for byte.
        if (line as usize) < 2 + 2 * self.pane_count {
            return target_for_line(line, self.pane_count);
        }
        let line = line as usize;
        if let Some(header) = self.agents_header() {
            if line == header {
                return LineTarget::None;
            }
            if line <= header + 2 * self.session_count {
                return LineTarget::SessionRow((line - header - 1) / 2);
            }
        }
        if let Some(header) = self.gates_header() {
            if line == header {
                return LineTarget::None;
            }
            if line <= header + 2 * self.gate_count {
                return LineTarget::GateRow((line - header - 1) / 2);
            }
        }
        LineTarget::None
    }
}

// The two sidebar presentation states a tab's swap set encodes: a 28-col
// rail or a 1-col sliver.
#[derive(Debug, Clone, Copy, PartialEq)]
enum DockState {
    Docked,
    Undocked,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ToggleAction {
    // One deliberate swap step: forwards (next) or backwards (previous)
    // through the installed [BASE, docked, undocked] order.
    SteerSwap { backwards: bool },
    // Rebuild the tab's swap set around its current pane arrangement, then
    // steer to the target state.
    RegenerateSwaps { target: DockState },
    Ignore,
}

#[derive(Debug, Clone, PartialEq)]
enum ClickAction {
    ToggleDock,
    FocusPane(u32),
    // Float subspace-tui on the gate's brief with --log pointed at its
    // decision log, so verdicts land on the gate's real record — the rail
    // itself never writes it.
    FloatGate { brief: String, log: String },
    None,
}

register_plugin!(Sidebar);

impl Sidebar {
    // Every PaneUpdate starts a new manifest generation. A TabUpdate from an
    // earlier generation must never authorize delivery after the pane moved,
    // was replaced, or became floating, so clear first and require a later
    // complete tab snapshot to re-arm.
    fn observe_agent_manifest(&mut self) {
        self.agent_manifest_generation = self.agent_manifest_generation.wrapping_add(1);
        self.agent_manifest_seen = true;
        self.agent_recipient = None;
    }

    // Derive this rail's stable server tab id from one complete TabUpdate.
    // `PaneManifest` keys are display positions; `TabInfo::tab_id` is the
    // stable identity returned by `new-tab`. Both the position and the id
    // must be unique in this snapshot or the broadcast remains inert.
    fn observe_agent_tab_update(&mut self, tabs: &[TabInfo]) {
        self.agent_recipient = None;
        if !self.agent_manifest_seen {
            return;
        }
        let Some(own_position) = self.own_tab else {
            return;
        };
        let mut own = tabs.iter().filter(|tab| tab.position == own_position);
        let Some(tab) = own.next() else {
            return;
        };
        if own.next().is_some() {
            return;
        }
        if tabs
            .iter()
            .filter(|candidate| candidate.tab_id == tab.tab_id)
            .count()
            != 1
        {
            return;
        }
        self.agent_recipient = Some(AgentRecipient {
            manifest_generation: self.agent_manifest_generation,
            own_position,
            stable_tab_id: tab.tab_id,
        });
    }

    // A pipe argument must be the canonical unsigned decimal spelling of a
    // native server tab id: 0 is valid, but leading zeroes, signs, whitespace,
    // and overflow are not alternate spellings that a sender can use.
    fn recipient_tab_id(args: &BTreeMap<String, String>) -> Option<usize> {
        let value = args.get("recipient-tab-id")?;
        if value.is_empty()
            || !value.as_bytes().iter().all(u8::is_ascii_digit)
            || (value.len() > 1 && value.starts_with('0'))
        {
            return None;
        }
        value.parse().ok()
    }

    // `agent-event` is a session-wide named-pipe broadcast. This receiver
    // guard is its only admission rule: no CWD, tab name, URL, pane id, or
    // display position fallback may create a row or a focus binding.
    fn accepts_agent_event(&self, args: &BTreeMap<String, String>) -> bool {
        let Some(armed) = self.agent_recipient else {
            return false;
        };
        let Some(configured_token) = self.config.get("recipient_token").filter(|token| !token.is_empty()) else {
            return false;
        };
        !self.own_floating
            && self.own_tab == Some(armed.own_position)
            && armed.manifest_generation == self.agent_manifest_generation
            && Self::recipient_tab_id(args) == Some(armed.stable_tab_id)
            && args.get("recipient-token") == Some(configured_token)
    }

    fn private_agent_pipe_name(&self, kind: &str) -> Option<String> {
        self.config
            .get("recipient_token")
            .filter(|token| !token.is_empty())
            .map(|token| format!("zaphod-agent-v1-{token}-{kind}"))
    }
}

impl ZellijPlugin for Sidebar {
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        self.plugin_id = get_plugin_ids().plugin_id;
        self.config = configuration;
        self.debug = debug_enabled(&self.config);
        self.wedge_poll_secs = wedge_poll_secs(&self.config);
        subscribe(&[
            EventType::PaneUpdate,
            EventType::TabUpdate,
            EventType::Mouse,
            EventType::Key,
            EventType::Timer,
            EventType::Visible,
            EventType::PermissionRequestResult,
        ]);
        // Permissions are requested on first render, not here: a request made
        // during tab construction races pane registration and gets parked in
        // the server's waiting-for-client queue, so the prompt never shows.
        set_timeout(STATUS_POLL_SECS);
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::PermissionRequestResult(status) => {
                self.permissions_granted = status == PermissionStatus::Granted;
                // Never take focus: clicks are delivered to the plugin without
                // focusing it (same mechanism as the built-in tab-bar). Only
                // after the grant — the permission prompt needs a focusable pane.
                if status == PermissionStatus::Granted && !self.nav_mode {
                    set_selectable(false);
                }
                self.route_toggle_to_self_if_active();
                true
            }
            Event::Visible(visible) => {
                if !visible {
                    self.toggle_route_requested = false;
                } else {
                    self.route_toggle_to_self_if_active();
                }
                false
            }
            Event::Key(key) if self.nav_mode => {
                if key.has_no_modifiers() {
                    match key.bare_key {
                        BareKey::Up | BareKey::Char('k') => {
                            self.nav_selected =
                                move_selection(self.nav_selected, -1, self.rows.len());
                        }
                        BareKey::Down | BareKey::Char('j') => {
                            self.nav_selected =
                                move_selection(self.nav_selected, 1, self.rows.len());
                        }
                        BareKey::Enter => {
                            let target = self.rows.get(self.nav_selected).map(|r| r.pane_id);
                            self.exit_nav(false);
                            if let Some(id) = target {
                                focus_terminal_pane(id, false, false);
                            }
                        }
                        BareKey::Esc => {
                            self.exit_nav(true);
                        }
                        _ => {}
                    }
                }
                true
            }
            Event::TabUpdate(tabs) => {
                self.reported_active_tab = tabs.iter().find(|t| t.active).map(|t| t.position);
                self.active_tab = self.reported_active_tab;
                self.tab_states = tabs
                    .iter()
                    .map(|t| {
                        (
                            t.position,
                            TabState {
                                id: t.tab_id,
                                swap_name: t.active_swap_layout_name.clone(),
                                swap_dirty: t.is_swap_layout_dirty,
                                floating_visible: t.are_floating_panes_visible,
                            },
                        )
                    })
                    .collect();
                self.observe_agent_tab_update(&tabs);
                if let Some(pending) = self.pending_steer {
                    let disposition =
                        pending_steer_disposition(pending, self.active_tab, &self.tab_states);
                    trace!(
                        self,
                        "steer disposition tab={} target={:?} reported_name={:?} dirty={} float={} active_tab={:?} -> {:?}",
                        pending.tab,
                        pending.target,
                        self.tab_states.get(&pending.tab).and_then(|s| s.swap_name.as_deref()),
                        self.tab_states.get(&pending.tab).is_some_and(|s| s.swap_dirty),
                        self.tab_states.get(&pending.tab).is_some_and(|s| s.floating_visible),
                        self.active_tab,
                        disposition
                    );
                    match disposition {
                        SteerDisposition::Fire { backwards } => {
                            self.pending_steer = None;
                            self.toggle_cooldown = Some((pending.tab, Instant::now()));
                            trace!(self, "steer fire backwards={}", backwards);
                            steer_swap(backwards);
                        }
                        SteerDisposition::Drop => self.pending_steer = None,
                        SteerDisposition::Keep => {}
                    }
                }
                self.route_toggle_to_self_if_active();
                false
            }
            Event::PaneUpdate(manifest) => {
                self.own_tab = own_tab_position(&manifest, self.plugin_id);
                self.observe_agent_manifest();
                self.instances = sidebar_instances(&manifest);
                let old = std::mem::take(&mut self.rows);
                self.rows = rows_for_own_tab(&manifest, self.plugin_id);
                preserve_agent_fields(&mut self.rows, &old);
                if let Some(own) = manifest
                    .panes
                    .values()
                    .flatten()
                    .find(|p| p.is_plugin && p.id == self.plugin_id)
                {
                    self.own_url = own.plugin_url.clone();
                    self.own_floating = own.is_floating;
                    // If focus ever lands on us outside nav mode (launch,
                    // layout focus), hand it back — but only when the tab has
                    // another selectable pane to receive it: bouncing in the
                    // session-birth window panics the whole server.
                    if should_hand_back_focus(own.is_focused, self.nav_mode, !self.rows.is_empty())
                    {
                        focus_previous_pane();
                    }
                }
                // A stale floating instance left behind by a previous layout
                // transition closes itself (fire-and-forget,
                // no permission gate on CloseSelf). close_self does not
                // remove the pane from the very next manifest snapshot, so
                // the flag makes the call one-shot rather than re-firing on
                // every PaneUpdate until the manifest catches up.
                if should_close_self(self.close_requested, self.plugin_id, self.own_floating, self.own_tab, &self.instances) {
                    self.close_requested = true;
                    trace!(self, "close_self firing own_tab={:?}", self.own_tab);
                    close_self();
                    return false;
                }
                self.route_toggle_to_self_if_active();
                // PaneUpdate fires constantly in agent-heavy tabs; re-rendering
                // a pinned overlay on every one makes the underlying panes
                // flicker. Only render when the derived view changed.
                self.rows != old || !self.rendered_once
            }
            Event::Timer(_) => {
                let changed =
                    if should_poll_statuses(self.own_tab, self.reported_active_tab, self.last_cols) {
                        self.refresh_statuses()
                    } else {
                        false
                    };
                set_timeout(STATUS_POLL_SECS);
                changed
            }
            Event::Mouse(Mouse::LeftClick(line, _col)) => {
                self.handle_click(line);
                false
            }
            _ => false,
        }
    }

    fn pipe(&mut self, pipe_message: PipeMessage) -> bool {
        trace!(self, "pipe recv name={}", pipe_message.name);
        if self.private_agent_pipe_name("ready").as_deref() == Some(pipe_message.name.as_str()) {
            if self.accepts_agent_event(&pipe_message.args) {
                if let PipeSource::Cli(pipe_id) = &pipe_message.source {
                    cli_pipe_output(pipe_id, "ready");
                }
            }
            return false;
        }
        if self.private_agent_pipe_name("snapshot").as_deref() == Some(pipe_message.name.as_str()) {
            if !self.accepts_agent_event(&pipe_message.args) {
                return false;
            }
            let Ok(changed) = apply_agent_snapshot(
                &mut self.sessions,
                &mut self.gates,
                pipe_message.payload.as_deref(),
            ) else {
                return false;
            };
            if let PipeSource::Cli(pipe_id) = &pipe_message.source {
                cli_pipe_output(pipe_id, "accepted");
            }
            return changed;
        }
        // Ordinary event callers need no response; Zellij auto-unblocks them
        // after this returns. Only the readiness branch above writes output.
        if self.private_agent_pipe_name("event").as_deref() == Some(pipe_message.name.as_str()) {
            if !self.accepts_agent_event(&pipe_message.args) {
                trace!(
                    self,
                    "agent-event dropped: recipient is absent, stale, or foreign"
                );
                return false;
            }
            // Only plugin state moves here — no host calls in pipe() (SPEC
            // landmine #4); true re-renders, false leaves the screen alone.
            let Some(payload) = pipe_message.payload.as_deref() else {
                trace!(self, "agent-event dropped: missing payload");
                return false;
            };
            return match parse_agent_event(payload) {
                Ok(event) => {
                    let changed = apply_agent_event(&mut self.sessions, &mut self.gates, event);
                    if let PipeSource::Cli(pipe_id) = &pipe_message.source {
                        cli_pipe_output(pipe_id, "accepted");
                    }
                    changed
                }
                Err(reason) => {
                    trace!(self, "agent-event dropped: {}", reason);
                    false
                }
            };
        }
        if pipe_message.name == "navigate" {
            // Nav belongs to the active tab's resident instance; every tab's
            // layout carries one.
            let active_tab = self.current_active_tab();
            if self.own_tab.is_some() && self.own_tab == active_tab {
                self.enter_nav();
                return true;
            }
            return false;
        }
        if pipe_message.name != "toggle" {
            return false;
        }
        // `reconfigure()` has no acknowledgement. Receipt of this direct
        // keybind pipe is the evidence that a runtime route reached us; only
        // the active tiled resident may consume it. Any other delivery is
        // inert and must not turn into deferred entry work.
        let active_tab = self.current_active_tab();
        if !should_accept_observed_toggle_pipe(
            self.permissions_granted,
            &pipe_message.source,
            self.own_floating,
            self.own_tab,
            active_tab,
        ) {
            trace!(self, "pipe toggle ignored: not an observed active tiled keybind route");
            return false;
        }
        trace!(self, "pipe toggle acting");
        self.perform_toggle();
        false
    }

    fn render(&mut self, _rows: usize, cols: usize) {
        self.rendered_once = true;
        self.last_cols = cols;
        if !self.permissions_requested {
            self.permissions_requested = true;
            request_permission(&permissions_for_config(&self.config));
        }
        if cols < STATUS_MIN_COLS {
            for line in sliver_lines(&self.rows, &self.sessions, &self.gates) {
                println!("{line}");
            }
            return;
        }
        // Header: any click runs the same dock toggle as Alt-/ (the ⇄ marks
        // it).
        println!(
            "\u{1b}[7m▾ PANES{}⇄ \u{1b}[0m",
            " ".repeat(cols.saturating_sub(10))
        );
        for (idx, row) in self.rows.iter().enumerate() {
            let title: String = row.title.chars().take(cols.saturating_sub(2)).collect();
            let mark = row_marker(row);
            if self.nav_mode && idx == self.nav_selected {
                println!("{}\u{1b}[7m{}\u{1b}[0m", mark, title); // nav selection
            } else {
                println!("{}{}{}\u{1b}[0m", mark, title_style(row), title);
            }
            let status = agent::status_line(&row.agent, cols.saturating_sub(4));
            println!("    \u{1b}[2m{}\u{1b}[0m", status);
        }
        // The agent-event sections render only when they have rows: a rail
        // that never receives an event prints exactly the lines above.
        if !self.sessions.is_empty() {
            println!(
                "\u{1b}[7m▾ AGENTS{}\u{1b}[0m",
                " ".repeat(cols.saturating_sub(9))
            );
            for session in &self.sessions {
                let bound = registered_session_pane(session, &self.rows).is_some();
                println!("{}", session_row_line(session, bound, cols));
                let summary: String =
                    session.summary.chars().take(cols.saturating_sub(4)).collect();
                println!("    \u{1b}[2m{}\u{1b}[0m", summary);
            }
        }
        if !self.gates.is_empty() {
            println!(
                "\u{1b}[7m▾ GATES{}\u{1b}[0m",
                " ".repeat(cols.saturating_sub(8))
            );
            for gate in &self.gates {
                println!("{}", gate_row_line(gate, cols));
                println!("    \u{1b}[2m{}\u{1b}[0m", gate_row_detail(gate, cols));
            }
        }
    }
}

impl Sidebar {
    fn route_toggle_to_self_if_active(&mut self) {
        if !should_route_toggle_to_self(
            self.toggle_route_requested,
            self.permissions_granted,
            self.own_floating,
            self.own_tab,
            self.active_tab,
        ) {
            return;
        }
        trace!(
            self,
            "installing current-client Alt / route for active tiled rail tab={:?}",
            self.own_tab
        );
        // `MessagePlugin` launches a pane when no matching instance exists.
        // This runtime-only `MessagePluginId` route names this already-loaded
        // pane directly, so a foreign tab can never bootstrap another one.
        reconfigure(runtime_toggle_keybind_kdl(self.plugin_id), false);
        self.toggle_route_requested = true;
    }

    fn current_active_tab(&mut self) -> Option<usize> {
        // get_focused_pane_info blocks on a response the host only writes
        // once ReadApplicationState is granted; pre-grant it panics inside
        // the shim, so fall back to the cached TabUpdate value.
        if !self.permissions_granted {
            return self.active_tab;
        }
        let active_tab =
            active_tab_for_decision(self.active_tab, get_focused_pane_info(), &self.tab_states);
        self.active_tab = active_tab;
        active_tab
    }

    fn enter_nav(&mut self) {
        self.nav_mode = true;
        self.return_focus = self.rows.iter().find(|r| r.focused).map(|r| r.pane_id);
        self.nav_selected = self
            .rows
            .iter()
            .position(|r| r.focused)
            .unwrap_or(0)
            .min(self.rows.len().saturating_sub(1));
        set_selectable(true);
        focus_plugin_pane(self.plugin_id, false, false);
    }

    fn exit_nav(&mut self, restore_focus: bool) {
        self.nav_mode = false;
        set_selectable(false);
        if restore_focus {
            if let Some(id) = self.return_focus.take() {
                focus_terminal_pane(id, false, false);
            }
        }
    }

    fn handle_click(&mut self, line: isize) {
        match decide_rail_click(line, &self.rows, &self.sessions, &self.gates, &self.pane_cwds) {
            ClickAction::ToggleDock => {
                // A local click is not a route acknowledgement. It remains a
                // rail-local convenience after the same visible resident has
                // requested its route; observed keybind pipes use the stricter
                // receipt-based gate above.
                if self.permissions_granted && self.toggle_route_requested {
                    self.perform_toggle();
                }
            }
            ClickAction::FocusPane(id) => {
                // A click-through during nav mode must also leave nav:
                // focus moves to the clicked pane, and a latched nav_mode
                // would keep the rail selectable with Enter/Esc routed to
                // the now-focused terminal.
                if self.nav_mode {
                    self.exit_nav(false);
                }
                focus_terminal_pane(id, false, false);
            }
            ClickAction::FloatGate { brief, log } => {
                if self.nav_mode {
                    self.exit_nav(false);
                }
                trace!(self, "float gate brief={} log={}", brief, log);
                // subspace-tui's --log implies persist: verdicts issued in
                // the floated TUI land on the gate's real decision log —
                // the rail itself never writes the record. Mouse handling
                // runs in update(), a context permitted to open panes
                // (SPEC landmine #4 bars pipe()/load() only).
                open_command_pane_floating(
                    CommandToRun {
                        path: "subspace-tui".into(),
                        args: vec![brief, "--log".to_owned(), log],
                        cwd: None,
                    },
                    None,
                    BTreeMap::new(),
                );
            }
            ClickAction::None => {
                // A gate row can decide to nothing: its log_path has no
                // derivable brief. Name the reason rather than floating a
                // wrong file.
                if let LineTarget::GateRow(idx) =
                    section_layout(self.rows.len(), self.sessions.len(), self.gates.len())
                        .target(line)
                {
                    trace!(
                        self,
                        "gate row {} click ignored: no brief derivable from log_path",
                        idx
                    );
                }
            }
        }
    }

    fn perform_toggle(&mut self) {
        // Callers establish authorization before arriving here. In particular,
        // a literal Alt / pipe is admitted only after we observe a keybind
        // delivery to this active tiled resident; a local request flag never
        // substitutes for that evidence.
        let active_tab = self.current_active_tab();
        trace!(
            self,
            "perform_toggle active_tab={:?} swap_name={:?} swap_dirty={} floating_visible={} own_tab={:?} own_floating={} pending_steer={:?} cooldown={:?}",
            active_tab,
            active_tab.and_then(|t| self.tab_states.get(&t)).and_then(|s| s.swap_name.as_deref()),
            active_tab.and_then(|t| self.tab_states.get(&t)).is_some_and(|s| s.swap_dirty),
            active_tab.and_then(|t| self.tab_states.get(&t)).is_some_and(|s| s.floating_visible),
            self.own_tab,
            self.own_floating,
            self.pending_steer,
            self.toggle_cooldown
        );
        if should_swallow_toggle(
            active_tab,
            self.pending_steer,
            self.toggle_cooldown,
            Instant::now(),
        ) {
            trace!(self, "perform_toggle swallowed tab={:?}", active_tab);
            return;
        }
        // A press that is not the parked steer's own bounce supersedes it.
        self.pending_steer = None;
        let active_state = active_tab.and_then(|tab| self.tab_states.get(&tab));
        let active_swap_name = active_state.and_then(|state| state.swap_name.clone());
        let active_swap_dirty = active_state.is_some_and(|state| state.swap_dirty);
        let active_floating_visible = active_state.is_some_and(|state| state.floating_visible);
        let action = decide_toggle(
            self.own_tab,
            self.own_floating,
            active_tab,
            active_swap_name.as_deref(),
            active_swap_dirty,
            active_floating_visible,
            self.plugin_id,
            &self.instances,
        );
        trace!(self, "decide_toggle verdict={:?}", action);
        match action {
            ToggleAction::SteerSwap { backwards } => {
                trace!(self, "action steer_swap backwards={}", backwards);
                // Flip docked <-> sliver by rearranging the tab's existing
                // panes; the plugin pane itself never hides or moves.
                steer_swap(backwards);
            }
            ToggleAction::RegenerateSwaps { target } => {
                trace!(self, "action regenerate_swaps target={:?}", target);
                self.regenerate_swaps(target);
            }
            ToggleAction::Ignore => trace!(self, "action ignore"),
        }
    }

    // Rebuild the tab's swap set around its current pane arrangement so the
    // toggle preserves the user's manual splits, then steer to the target
    // state. Any missing input degrades to plain cycling: on a damaged tab
    // zellij's first call re-applies the current template and the second
    // advances — the arrangement snap-folds, but the toggle still lands.
    fn regenerate_swaps(&mut self, target: DockState) {
        let rebuilt = rebuild_target(self.permissions_granted, self.own_tab, &self.tab_states)
            .and_then(|(tab, tab_id)| self.install_split_preserving_swaps(tab, tab_id, target));
        if rebuilt.is_none() {
            trace!(self, "action fallback_swap_cycle target={:?}", target);
            fallback_swap_cycle();
        }
    }

    // The shared dump → transform → override machinery behind regeneration:
    // rebuild the tab's
    // arrangement and record the steer that completes the toggle once
    // TabUpdate reports the new set installed. Some means the tab is
    // handled — either the swap set was installed, or the rebuild was
    // abandoned because the dump already carries another instance's sidebar.
    // A second override would corrupt the tab by re-absorbing its panes.
    // None means the rebuild could not run and the caller degrades.
    fn install_split_preserving_swaps(
        &mut self,
        tab: usize,
        tab_id: usize,
        target: DockState,
    ) -> Option<()> {
        let Some(url) = self.own_url.clone() else {
            trace!(self, "rebuild failed tab={}: own url unknown", tab);
            return None;
        };
        // In-band errors and a 1s server-side timeout; a stale tab id dumps
        // no tab node and fails the rebuild below.
        let (dump, _metadata) = match dump_session_layout_for_tab(tab_id) {
            Ok(dump) => dump,
            Err(e) => {
                trace!(self, "rebuild failed tab={} tab_id={}: dump error: {}", tab, tab_id, e);
                return None;
            }
        };
        // The dump is authoritative current state, immune to the PaneUpdate
        // instance-list lag that lets a remote election re-fire on a tab
        // whose freshly installed resident it has not yet seen. If a sidebar
        // already lives in the tiled region, the resident owns the tab.
        if dump_contains_sidebar(&dump, &url) {
            trace!(
                self,
                "rebuild aborted tab={}: dump already carries a sidebar (resident owns it)",
                tab
            );
            return Some(());
        }
        let layout = match split_preserving_layout_kdl(&dump, &url, &self.config) {
            Ok(layout) => layout,
            Err(e) => {
                trace!(self, "rebuild failed tab={}: transform error: {}", tab, e);
                return None;
            }
        };
        trace!(
            self,
            "action install_split_preserving_swaps tab={} tab_id={} target={:?}",
            tab,
            tab_id,
            target
        );
        override_layout(
            LayoutInfo::Stringified(layout),
            true, // retain existing terminal panes
            true, // retain existing plugin panes
            true, // apply only to the active tab
            BTreeMap::new(),
        );
        // Where the override lands is zellij's call, not ours: the server
        // relayouts the tab right after installing the set, advancing it
        // to the first fitting entry — BASE only when the absorb base's
        // exact pane count matches (single-shell tabs), "docked"
        // otherwise. On the tab this actor lives in, the recorded target
        // lets the TabUpdate handler finish the toggle from whichever entry
        // is reported. On a remote tab the steer is dropped: it would act
        // on the wrong tab (swap steps follow the client's active tab), the
        // relayout already lands the tab docked, and the resident installed
        // here owns every later toggle.
        if steer_completes_locally(self.own_tab, tab) {
            self.pending_steer = Some(PendingSteer { tab, target });
            trace!(self, "pending_steer recorded tab={} target={:?}", tab, target);
        } else {
            trace!(
                self,
                "remote rebuild tab={} target={:?}: no steer armed (resident owns arrival)",
                tab,
                target
            );
        }
        Some(())
    }

    // Returns whether any row's agent fields or polled cwd changed (i.e. a
    // render is due — a cwd change can flip a session row's binding).
    fn refresh_statuses(&mut self) -> bool {
        let mut backoff = std::mem::take(&mut self.poll_backoff);
        let live: std::collections::BTreeSet<u32> = self.rows.iter().map(|r| r.pane_id).collect();
        backoff.retain(|pane_id, _| live.contains(pane_id));
        self.pane_cwds.retain(|pane_id, _| live.contains(pane_id));
        let mut changed = false;
        for row in self.rows.iter_mut() {
            let state = backoff.entry(row.pane_id).or_default();
            if !state.due() {
                continue; // backed off: keep the previous status
            }
            let pane_id = PaneId::Terminal(row.pane_id);
            let started = Instant::now();
            let command = match self.wedge_poll_secs {
                Some(secs) => {
                    trace!(self, "wedge drill: sleeping {}s in place of pane {} status call", secs, row.pane_id);
                    std::thread::sleep(Duration::from_secs(secs));
                    Err("wedge drill".to_owned())
                }
                None => get_pane_running_command(pane_id),
            };
            if wedge_aborts_pass(started.elapsed()) {
                // The wedged pane earned its backoff by stalling the pass,
                // whatever its call returned; the untried panes keep their
                // previous status AND their backoff state — stale-not-blank,
                // the posture should_poll_statuses already chose.
                state.record(true);
                trace!(
                    self,
                    "status pass aborted: pane {} wedge-classified after {}ms",
                    row.pane_id,
                    started.elapsed().as_millis()
                );
                break;
            }
            // The pane's cwd feeds session binding: one more timed call
            // under the same wedge budget. Its twin get_pane_running_command
            // produced the observed ~13s wedges despite the export's 100ms
            // guard, so the guard is not trusted here either. A failed call
            // keeps the previous entry (stale-not-blank).
            let cwd_started = Instant::now();
            let cwd = get_pane_cwd(pane_id);
            if wedge_aborts_pass(cwd_started.elapsed()) {
                state.record(true);
                trace!(
                    self,
                    "status pass aborted: pane {} cwd call wedge-classified after {}ms",
                    row.pane_id,
                    cwd_started.elapsed().as_millis()
                );
                break;
            }
            if let Ok(cwd) = cwd {
                if self.pane_cwds.get(&row.pane_id) != Some(&cwd) {
                    self.pane_cwds.insert(row.pane_id, cwd);
                    changed = true;
                }
            }
            let viewport = get_pane_scrollback(pane_id, false).map(|contents| contents.viewport);
            state.record(command.is_err());
            let enriched = agent::enrich_fields(&row.agent, &row.title, command, viewport);
            if enriched != row.agent {
                row.agent = enriched;
                changed = true;
            }
        }
        self.poll_backoff = backoff;
        changed
    }
}

// The persistent configuration binds Alt / to NoOp. Once a tiled rail is
// actually visible, that rail upgrades only its current client's runtime
// binding to its existing plugin id. Zellij's MessagePluginId action routes
// directly and therefore has no launch-if-missing behavior.
fn runtime_toggle_keybind_kdl(plugin_id: u32) -> String {
    format!(
        "keybinds {{\n    shared {{\n        bind \"Alt /\" {{\n            MessagePluginId {plugin_id} {{\n                name \"toggle\"\n            }}\n        }}\n    }}\n}}\n"
    )
}

fn should_route_toggle_to_self(
    route_requested: bool,
    permissions_granted: bool,
    own_floating: bool,
    own_tab: Option<usize>,
    active_tab: Option<usize>,
) -> bool {
    !route_requested
        && permissions_granted
        && !own_floating
        && own_tab.is_some()
        && own_tab == active_tab
}

// A runtime route has no acknowledgement. For a literal Alt /, the first
// trustworthy evidence is the keybind pipe itself. It is still safe only when
// the recipient is the active tab's tiled rail; a CLI/plugin pipe, floating
// rail, unknown tab, or stale active-tab view must remain inert.
fn should_accept_observed_toggle_pipe(
    permissions_granted: bool,
    source: &PipeSource,
    own_floating: bool,
    own_tab: Option<usize>,
    active_tab: Option<usize>,
) -> bool {
    permissions_granted
        && matches!(source, PipeSource::Keybind)
        && !own_floating
        && own_tab.is_some()
        && own_tab == active_tab
}

// A routed pipe reaches one already-running sidebar instance. Only the
// active tab's tiled resident may act.
// - The active tab's tiled resident steers the tab's swap layout one step
//   toward the other dock state, by name — never by blind cycling, whose
//   position semantics around BASE and the list end are unreliable.
// - When the tab is damaged (manual split/resize), a swap step would
//   snap-fold the user's arrangement into a stale template; the resident
//   instead rebuilds the swap set around the current arrangement.
fn decide_toggle(
    own_tab: Option<usize>,
    own_floating: bool,
    active_tab: Option<usize>,
    active_swap_name: Option<&str>,
    active_swap_dirty: bool,
    active_floating_visible: bool,
    _own_pane_id: u32,
    _instances: &[SidebarInstance],
) -> ToggleAction {
    let Some(active) = active_tab else {
        return ToggleAction::Ignore;
    };
    if own_tab == Some(active) && !own_floating {
        // Only "docked" (position 1 of [BASE, docked, undocked]) steers
        // forward, to undocked. Everything else steers back: undocked one
        // step to docked, and BASE — geometrically docked on template-born
        // tabs, so a forward step there is a dead press — wraps from
        // position 0 to the list end (undocked, a visible collapse);
        // zellij's previous-swap wrap is deterministic while next past the
        // end is not (swap_layouts.rs progress_layout!).
        // A foreign swap name — a builtin or user-captured set, not ours —
        // must not be stepped through: cycling it re-tiles the user's panes
        // while the dock never toggles, and pending_steer_disposition drops
        // a foreign report. Rebuild the swap set instead, like a dirty tab.
        let foreign_swap =
            active_swap_name.is_some_and(|name| !matches!(name, "BASE" | "docked" | "undocked"));
        if active_floating_visible {
            // While the tab's floating panes are visible, swap_name and
            // swap_dirty speak for the FLOATING layer (TabState) and a swap
            // step would cycle that layer; the tiled dock state is
            // unreadable. The press is dropped rather than parked — a parked
            // press would collapse the dock whenever the floats next hide.
            ToggleAction::Ignore
        } else if active_swap_dirty || foreign_swap {
            ToggleAction::RegenerateSwaps {
                target: if active_swap_name == Some("undocked") {
                    DockState::Docked
                } else {
                    DockState::Undocked
                },
            }
        } else {
            ToggleAction::SteerSwap {
                backwards: active_swap_name != Some("docked"),
            }
        }
    } else {
        // A floating rail, a remote rail, or no rail at all is not a toggle
        // target. The entry command owns initialization; Alt / never does.
        ToggleAction::Ignore
    }
}

// Whether a split-preserving swap rebuild can run against a tab, and the
// server tab id to dump from if so. The dump host call blocks on a
// response that only exists post-grant (it panics in the shim pre-grant),
// and it speaks the server's stable tab id, which only the
// TabUpdate-derived states can translate a display position into. None:
// the rebuild cannot run and the caller degrades.
fn rebuild_target(
    permissions_granted: bool,
    tab: Option<usize>,
    tab_states: &BTreeMap<usize, TabState>,
) -> Option<(usize, usize)> {
    if !permissions_granted {
        return None;
    }
    let tab = tab?;
    let id = tab_states.get(&tab)?.id;
    Some((tab, id))
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum SteerDisposition {
    // One swap step in the given direction through the installed
    // [BASE, docked, undocked] order; previous wraps position 0 to the
    // list end deterministically, only forward past the end is unreliable
    // (swap_layouts.rs progress_layout!).
    Fire { backwards: bool },
    Keep,
    Drop,
}

// A steered press recorded alongside a swap-set override waits for the
// server to report where the override landed. zellij relayouts the tab
// right after installing the set, so the report is "BASE" only when the
// absorb base's exact pane count matches (single-shell tabs); multi-pane
// tabs arrive already at "docked". The steer converts the reported entry
// into the one deliberate step that reaches the target — or stands down
// when the relayout already landed there, since one more step would
// overshoot it (the live one-step-off arrival). While floating panes are
// visible the reported name speaks for the floating layer, whose birth
// entry is also "BASE"; the tiled outcome is unreadable, so the press
// stays armed. The dirty flag is no part of the signature — a landed
// override was observed live still flagged dirty. Swap presses act on the
// client's active tab, so the press is abandoned when the tab is gone or
// no longer active — the regenerated set stays installed and a later
// press on the tab steers by name. A foreign name means the override
// failed or raced, and the press is abandoned rather than kept armed
// forever; only a nameless report (the one-selectable-pane blind spot)
// keeps it waiting.
fn pending_steer_disposition(
    steer: PendingSteer,
    active_tab: Option<usize>,
    tab_states: &BTreeMap<usize, TabState>,
) -> SteerDisposition {
    let Some(state) = tab_states.get(&steer.tab) else {
        return SteerDisposition::Drop;
    };
    if active_tab != Some(steer.tab) {
        return SteerDisposition::Drop;
    }
    if state.floating_visible {
        return SteerDisposition::Keep;
    }
    match (state.swap_name.as_deref(), steer.target) {
        (Some("BASE"), DockState::Docked) => SteerDisposition::Fire { backwards: false },
        (Some("BASE"), DockState::Undocked) => SteerDisposition::Fire { backwards: true },
        (Some("docked"), DockState::Undocked) => SteerDisposition::Fire { backwards: false },
        (Some("undocked"), DockState::Docked) => SteerDisposition::Fire { backwards: true },
        (Some(_), _) => SteerDisposition::Drop,
        (None, _) => SteerDisposition::Keep,
    }
}

// Whether trace instrumentation is enabled: the "debug" config key present
// with a non-empty value.
fn debug_enabled(config: &BTreeMap<String, String>) -> bool {
    config.get("debug").is_some_and(|value| !value.is_empty())
}

// Wedge drill knob: the "wedge_poll_secs" config key, parsed as whole
// seconds. When set, each status poll sleeps this long in place of its
// get_pane_running_command call — the same blocking layer (the plugin's
// pinned thread) as a real wedge, so the wedge budget is exercisable end to
// end without waiting to catch a wild one. Absent or unparseable: off.
fn wedge_poll_secs(config: &BTreeMap<String, String>) -> Option<u64> {
    config.get("wedge_poll_secs").and_then(|value| value.parse().ok())
}

// Number of timers to skip after a pane's status poll fails: 0, 1, 3, 7, 15,
// then flat at 15 (~30s at a 2s poll). Caps the shift so it never overflows.
fn backoff_skips(failures: u32) -> u32 {
    (1u32 << failures.min(4)) - 1
}

// Whether one pane-status call stalled long enough to abort the rest of the
// poll pass. The plugin's pinned thread runs its queue FIFO, so a CLI pipe
// dispatched at this instance waits behind the in-flight pass; bounding the
// pass at one wedge bounds that wait at ≈ one wedge instead of one per
// remaining pane.
fn wedge_aborts_pass(elapsed: Duration) -> bool {
    elapsed >= WEDGE_THRESHOLD
}

// Whether this instance should poll pane statuses at all. Only the active
// tab's docked rail is on screen: an undocked sliver (render width below
// STATUS_MIN_COLS) shows no status, and a background tab's rail is off
// screen, so both skip — this collapses N per-tab pollers to the one visible
// instance. cols is the live render width, accurate for the visible sidebar.
// When the active tab is unknown, bias toward polling: a skipped poll only
// leaves the previous status on screen (never blank), so a wasted poll is
// cheaper than briefly stale status after a fast tab switch.
fn should_poll_statuses(own_tab: Option<usize>, active_tab: Option<usize>, cols: usize) -> bool {
    if cols < STATUS_MIN_COLS {
        return false;
    }
    match (own_tab, active_tab) {
        (Some(own), Some(active)) => own == active,
        _ => true,
    }
}

// A deferred steer completes only for the actor that lives in the tab it
// rebuilt: previous/next_swap_layout act on the client's active tab, so a
// remote actor (the election leader rebuilding a sidebar-less tab it does
// not live in) can never fire the steer on the right tab. The override's
// own relayout lands that tab docked with its splits preserved, and the
// freshly installed resident owns every later toggle — so a remote actor
// arms no steer. A held one only jams it: should_swallow_toggle then eats
// its presses for that tab until a press on another tab supersedes it.
fn steer_completes_locally(own_tab: Option<usize>, rebuilt_tab: usize) -> bool {
    own_tab == Some(rebuilt_tab)
}

// A toggle press bounces when it repeats into the same tab's JIT window:
// the pipeline (dump → override → deferred steer) makes its collapse
// visible only a beat after the press, so a quick second press reads the
// tab as needing another toggle and undoes the one still landing. Swallow
// a press for the tab whose steer is still parked, and for the tab whose
// steer fired less than TOGGLE_COOLDOWN ago; a press for any other tab is
// a fresh intent.
fn should_swallow_toggle(
    active_tab: Option<usize>,
    pending_steer: Option<PendingSteer>,
    cooldown: Option<(usize, Instant)>,
    now: Instant,
) -> bool {
    let Some(tab) = active_tab else {
        return false;
    };
    pending_steer.is_some_and(|steer| steer.tab == tab)
        || cooldown.is_some_and(|(cooled, fired)| {
            cooled == tab && now.duration_since(fired) < TOGGLE_COOLDOWN
        })
}

fn steer_swap(backwards: bool) {
    if backwards {
        previous_swap_layout();
    } else {
        next_swap_layout();
    }
}

// Dirty-tab cycling without a regenerated swap set: zellij's first call
// only re-applies the current template (consuming the damage), the second
// advances.
fn fallback_swap_cycle() {
    next_swap_layout();
    next_swap_layout();
}

// Any click on the header — the ⇄ control or the title text — runs the same
// dock toggle as Alt-/; row clicks focus their pane.
fn decide_click(line: isize, rows: &[Row]) -> ClickAction {
    match target_for_line(line, rows.len()) {
        LineTarget::Header => ClickAction::ToggleDock,
        LineTarget::Row(idx) => rows
            .get(idx)
            .map(|row| ClickAction::FocusPane(row.pane_id))
            .unwrap_or(ClickAction::None),
        // The pane-rows map never yields section rows.
        LineTarget::None | LineTarget::SessionRow(_) | LineTarget::GateRow(_) => ClickAction::None,
    }
}

// Click decision over the whole sectioned rail. Pane-region lines defer to
// decide_click (the shipped pane-rows decider); a session row focuses its
// cwd-bound pane and an unbound row's click is dead — never guessed; a gate
// row floats the review TUI on the gate's brief, and a log path with no
// derivable brief clicks to nothing rather than floating a wrong file.
fn decide_rail_click(
    line: isize,
    rows: &[Row],
    sessions: &[SessionEvent],
    gates: &[GateEvent],
    _cwds: &BTreeMap<u32, PathBuf>,
) -> ClickAction {
    match section_layout(rows.len(), sessions.len(), gates.len()).target(line) {
        LineTarget::Header | LineTarget::Row(_) => decide_click(line, rows),
        LineTarget::SessionRow(idx) => sessions
            .get(idx)
            .and_then(|session| registered_session_pane(session, rows))
            .map(ClickAction::FocusPane)
            .unwrap_or(ClickAction::None),
        LineTarget::GateRow(idx) => gates
            .get(idx)
            .and_then(|gate| {
                brief_path_for_log(&gate.log_path).map(|brief| ClickAction::FloatGate {
                    brief,
                    log: gate.log_path.clone(),
                })
            })
            .unwrap_or(ClickAction::None),
        LineTarget::None => ClickAction::None,
    }
}

// A floating instance is superseded once its own tab holds a tiled sidebar:
// it would linger as an invisible config-matched zombie that keeps receiving
// pipes. Both facts come from the same manifest snapshot, so a transient
// layout change cannot half-match.
fn is_stray_floating_bootstrap(
    own_floating: bool,
    own_tab: Option<usize>,
    instances: &[SidebarInstance],
) -> bool {
    let Some(tab) = own_tab else {
        return false;
    };
    own_floating && instances.iter().any(|i| i.tab == tab && !i.floating)
}

// If a malformed or legacy layout leaves two tiled sidebars in one tab, the
// higher-id one is redundant and closes itself, leaving the single lowest-id
// resident. Both read the same manifest snapshot, so the choice is
// deterministic. A floating sidebar takes the stray-instance path instead.
fn is_redundant_tiled_sidebar(
    own_pane_id: u32,
    own_tab: Option<usize>,
    own_floating: bool,
    instances: &[SidebarInstance],
) -> bool {
    let Some(tab) = own_tab else {
        return false;
    };
    !own_floating
        && instances
            .iter()
            .any(|i| i.tab == tab && !i.floating && i.pane_id < own_pane_id)
}

// Gates close_self to one call per instance lifetime: the manifest can still
// show the closing shape on the PaneUpdate right after the call, since the
// host has not yet dropped the pane, and a second close_self would fire on
// every such snapshot until it does. An instance closes when it is a stray
// floating instance superseded by a tiled rail, or a redundant tiled rail
// behind a lower-id sibling.
fn should_close_self(
    already_requested: bool,
    own_pane_id: u32,
    own_floating: bool,
    own_tab: Option<usize>,
    instances: &[SidebarInstance],
) -> bool {
    !already_requested
        && (is_stray_floating_bootstrap(own_floating, own_tab, instances)
            || is_redundant_tiled_sidebar(own_pane_id, own_tab, own_floating, instances))
}

// Handing focus back with nowhere to send it panics the server in the
// session-birth window (get_active_pane_id unwraps None); only bounce when
// the manifest shows another selectable pane in our tab.
fn should_hand_back_focus(own_focused: bool, nav_mode: bool, has_focus_target: bool) -> bool {
    own_focused && !nav_mode && has_focus_target
}

// The sidebar's own slot in a generated layout. The plugin block repeats
// the live instance's URL and configuration exactly, so layout application
// re-seats the existing pane instead of spawning a second instance.
fn rail_pane_kdl(plugin_url: &str, config: &BTreeMap<String, String>, cols: usize) -> String {
    let config_lines: String = config
        .iter()
        .map(|(key, value)| format!("                {key} \"{value}\"\n"))
        .collect();
    format!(
        r#"        pane size={cols} borderless=true name="sidebar" {{
            plugin location="{plugin_url}" {{
{config_lines}            }}
        }}
"#
    )
}

// Builds the layout regenerate_swaps overrides a damaged tab with: the base
// absorbs every pane into a rail + stacked children tab (the only
// retained-pane-correct shape for an override), while the docked/undocked
// swaps carry the tab's dumped arrangement so the follow-up steer re-seats
// the user's splits with only the rail width changed. The dump arrives with
// the requesting plugin's own rail already removed by the server. Chrome
// panes are extracted from wherever the dump seats them — zellij's absorb
// can bake the tab-bar into the user region, and a baked copy would ride
// into the swaps as a user pane and mangle the tab permanently — and
// re-emitted as canonical rows, exactly one node per chrome pane the tab
// actually has: a node without a matching pane wedges the tab (swaps never
// spawn), a pane without a node gets absorbed again.
fn split_preserving_layout_kdl(
    dump: &str,
    plugin_url: &str,
    config: &BTreeMap<String, String>,
) -> Result<String, String> {
    let body = extract_tab_body(dump)?;
    let mut chrome = Vec::new();
    let region: Vec<Vec<String>> = top_level_blocks(&body)
        .into_iter()
        .filter(|block| !is_floating_panes_block(block))
        .map(|block| {
            block
                .iter()
                .map(|line| {
                    remove_prop(
                        &remove_prop(&remove_prop(line, "focus"), "name"),
                        "borderless",
                    )
                })
                .collect()
        })
        .filter_map(|block| extract_chrome_panes(block, plugin_url, &mut chrome))
        .collect();
    if region.is_empty() {
        return Err("dumped tab has no panes besides chrome".to_owned());
    }

    let region_kdl = if region.len() == 1 {
        // A single container fills whatever remains next to the rail; its
        // dumped size described the tab before the rail was removed.
        let mut lines = region[0].clone();
        lines[0] = remove_prop(&lines[0], "size");
        lines.join("\n") + "\n"
    } else {
        // Sibling rows keep their relative sizes inside one flexible
        // container (default split direction: top-to-bottom, as in the tab).
        format!(
            "pane {{\n{}\n}}\n",
            region.iter().flatten().cloned().collect::<Vec<_>>().join("\n")
        )
    };
    let chrome_row = |location: &String| {
        format!("pane size=1 borderless=true {{\nplugin location=\"{location}\"\n}}\n")
    };
    let is_top_bar = |location: &&String| location.as_str() == "zellij:tab-bar";
    let chrome_top: String = chrome.iter().filter(is_top_bar).map(chrome_row).collect();
    let chrome_bottom: String = chrome
        .iter()
        .filter(|location| !is_top_bar(location))
        .map(chrome_row)
        .collect();
    let tab_kdl = |cols: usize, main: &str| {
        format!(
            "tab {{\n{chrome_top}pane split_direction=\"vertical\" {{\n{rail}{main}}}\n{chrome_bottom}}}\n",
            rail = rail_pane_kdl(plugin_url, config, cols),
        )
    };
    Ok(format!(
        "layout {{\nswap_tiled_layout name=\"docked\" {{\n{docked}}}\nswap_tiled_layout name=\"undocked\" {{\n{undocked}}}\n{base}}}\n",
        docked = tab_kdl(DOCKED_COLS, &region_kdl),
        undocked = tab_kdl(UNDOCKED_COLS, &region_kdl),
        base = tab_kdl(DOCKED_COLS, "pane stacked=true {\nchildren\n}\n"),
    ))
}

// Whether the dumped tab already carries a sidebar rail in its tiled region
// — a resident another instance installed. The server strips only the
// requesting plugin's own pane from a dump
// (populate_session_layout_metadata), so any sidebar pane surviving here
// belongs to a different instance: its resident owns the tab. The floating
// layer is skipped — a transient floater does not own the tiled arrangement.
fn dump_contains_sidebar(dump: &str, own_url: &str) -> bool {
    let Ok(body) = extract_tab_body(dump) else {
        return false;
    };
    top_level_blocks(&body)
        .iter()
        .filter(|block| !is_floating_panes_block(block))
        .any(|block| block_has_sidebar(block, own_url))
}

fn block_has_sidebar(block: &[String], own_url: &str) -> bool {
    if let Some(location) = plugin_location(block) {
        return classify_plugin_location(&location, own_url) == PluginRole::Rail;
    }
    if block.len() < 2 {
        // zellij's dump serializes a single-child rail pane with its plugin
        // inline on one line (see extract_chrome_panes).
        return inline_plugin_location(&block[0]).is_some_and(|location| {
            classify_plugin_location(&location, own_url) == PluginRole::Rail
        });
    }
    top_level_blocks(&block[1..block.len() - 1])
        .iter()
        .any(|child| block_has_sidebar(child, own_url))
}

// Lines between the first depth-1 `tab` node's braces in a dumped layout.
// The dump is machine-generated multi-line KDL, so tracking brace depth per
// line (ignoring braces inside quoted strings) is structure-exact.
fn extract_tab_body(dump: &str) -> Result<Vec<String>, String> {
    let mut depth = 0isize;
    let mut capturing = false;
    let mut body = Vec::new();
    for line in dump.lines() {
        let trimmed = line.trim();
        let delta = brace_delta(line);
        if capturing {
            if depth + delta <= 1 {
                return Ok(body);
            }
            body.push(line.to_owned());
        } else if depth == 1 && (trimmed == "tab" || trimmed.starts_with("tab ")) {
            if delta != 1 {
                return Err("unsupported tab node shape in dump".to_owned());
            }
            capturing = true;
        }
        depth += delta;
    }
    Err("no tab node in dump".to_owned())
}

// Splits a node body into its immediate child nodes, each a run of lines.
fn top_level_blocks(body: &[String]) -> Vec<Vec<String>> {
    let mut blocks = Vec::new();
    let mut current = Vec::new();
    let mut depth = 0isize;
    for line in body {
        current.push(line.clone());
        depth += brace_delta(line);
        if depth <= 0 {
            blocks.push(std::mem::take(&mut current));
            depth = 0;
        }
    }
    if !current.is_empty() {
        blocks.push(current);
    }
    blocks
}

// Removes every built-in bar pane (tab-bar/status-bar/compact-bar) and
// every copy of the sidebar's own rail from a dumped pane block, wherever
// zellij's absorb seated them, recording each removed chrome location
// (rail copies are dropped without recording: rail_pane_kdl always
// contributes the canonical slot, so a dumped copy is never re-emitted).
// A container emptied by the removal is dropped with it; a container left
// with a single child is replaced by that child, which takes over the
// container's slot. A pane wrapping any other plugin — third-party or a
// builtin region pane like strider — is left intact.
fn extract_chrome_panes(
    block: Vec<String>,
    own_plugin_url: &str,
    removed: &mut Vec<String>,
) -> Option<Vec<String>> {
    if block.len() < 2 {
        // A pane can serialize with its plugin child inline on one line:
        //   pane size=1 borderless=true { plugin location="zellij:tab-bar" }
        // zellij's dump writes chrome (and single-child rails) this way, so
        // classify it like a multi-line chrome/rail pane; a plain leaf pane
        // (no inline plugin) is kept unchanged.
        if let Some(location) = inline_plugin_location(&block[0]) {
            return match classify_plugin_location(&location, own_plugin_url) {
                PluginRole::Chrome => {
                    removed.push(location);
                    None
                }
                PluginRole::Rail => None,
                PluginRole::ThirdParty => Some(block),
            };
        }
        return Some(block);
    }
    let children = top_level_blocks(&block[1..block.len() - 1]);
    if let Some(location) = children.iter().find_map(|child| plugin_location(child)) {
        return match classify_plugin_location(&location, own_plugin_url) {
            PluginRole::Chrome => {
                removed.push(location);
                None
            }
            PluginRole::Rail => None,
            PluginRole::ThirdParty => Some(block),
        };
    }
    if children.is_empty() {
        return Some(block);
    }
    let child_count = children.len();
    let kept: Vec<Vec<String>> = children
        .into_iter()
        .filter_map(|child| extract_chrome_panes(child, own_plugin_url, removed))
        .collect();
    if kept.is_empty() {
        return None;
    }
    if kept.len() == 1 && child_count > 1 {
        let mut child = kept.into_iter().next().unwrap();
        child[0] = fill_container_slot(&child[0], &block[0]);
        return Some(child);
    }
    let mut rebuilt = vec![block[0].clone()];
    rebuilt.extend(kept.into_iter().flatten());
    rebuilt.push(block[block.len() - 1].clone());
    Some(rebuilt)
}

// The plugin location of a pane whose plugin child is inlined on the same
// line: `pane ... { plugin location="X" ... }`. prop_token matches the first
// UNQUOTED `location="..."` — the plugin's, since panes carry no location
// prop — even past a nested config block like the rail's `{ rail "1" }`. A
// plain leaf pane (no inline plugin) has no location and returns None.
fn inline_plugin_location(line: &str) -> Option<String> {
    let token = prop_token(line, "location")?;
    let location = token.strip_prefix("location=\"")?.strip_suffix('"')?;
    Some(location.to_owned())
}

// The location of a plugin node, when the block is one directly (its first
// line is a bare `plugin location="..."` node, not a pane wrapping one).
fn plugin_location(block: &[String]) -> Option<String> {
    let first = block.first()?;
    let trimmed = first.trim_start();
    if trimmed != "plugin" && !trimmed.starts_with("plugin ") {
        return None;
    }
    let token = prop_token(first, "location")?;
    let location = token.strip_prefix("location=\"")?.strip_suffix('"')?;
    Some(location.to_owned())
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum PluginRole {
    // A zellij built-in bar (tab-bar/status-bar/compact-bar): extracted and
    // re-emitted as a canonical chrome row.
    Chrome,
    // A copy of the sidebar's own rail, wherever it landed in the region:
    // dropped outright, since rail_pane_kdl always supplies the canonical
    // slot.
    Rail,
    // Any other plugin — a user's, or a builtin region pane like strider:
    // not ours to remove.
    ThirdParty,
}

// Chrome is an allowlist of zellij's builtin bars, not `zellij:*` wholesale:
// the other builtins (strider, session-manager, …) are full panes, and
// misreading one as chrome re-emits it as a size=1 bar — destroying it —
// while misreading a future bar as ThirdParty only leaves it riding along
// in the region. A defensive substring match backs up the exact-URL check:
// a rail built from a different path or an older/newer configuration (same
// plugin, different identity string) must still be recognized.
fn classify_plugin_location(location: &str, own_plugin_url: &str) -> PluginRole {
    if matches!(
        location,
        "zellij:tab-bar" | "zellij:status-bar" | "zellij:compact-bar"
    ) {
        PluginRole::Chrome
    } else if location == own_plugin_url || location.contains("zellij-sidebar") {
        PluginRole::Rail
    } else {
        PluginRole::ThirdParty
    }
}

// Rewrites a hoisted child's header to occupy its old container's slot:
// the container's size replaces the child's own, and stack-only flags die
// with the stack.
fn fill_container_slot(child_header: &str, container_header: &str) -> String {
    let stripped = remove_prop(&remove_prop(child_header, "size"), "expanded");
    let Some(size) = prop_token(container_header, "size") else {
        return stripped;
    };
    let trimmed = stripped.trim_end();
    match trimmed.strip_suffix('{') {
        Some(head) => format!("{} {size} {{", head.trim_end()),
        None => format!("{trimmed} {size}"),
    }
}

fn is_floating_panes_block(block: &[String]) -> bool {
    block
        .first()
        .is_some_and(|l| l.trim().split_whitespace().next() == Some("floating_panes"))
}

// Net brace-depth change of one KDL line; braces inside quoted strings
// (pane titles, cwds) do not count.
fn brace_delta(line: &str) -> isize {
    let mut delta = 0;
    let mut in_string = false;
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' if in_string => {
                chars.next();
            }
            '"' => in_string = !in_string,
            '{' if !in_string => delta += 1,
            '}' if !in_string => delta -= 1,
            _ => {}
        }
    }
    delta
}

// Char span of the first `key=value` property in a KDL line — quoted or
// bare values — matching only outside quoted strings, so a cwd or title
// that happens to contain `key=` is left alone.
fn prop_span(chars: &[char], key: &str) -> Option<(usize, usize)> {
    let pattern: Vec<char> = format!("{key}=").chars().collect();
    let mut in_string = false;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if in_string {
            if c == '\\' {
                i += 2;
                continue;
            }
            if c == '"' {
                in_string = false;
            }
            i += 1;
            continue;
        }
        let at_token_start = i > 0 && chars[i - 1].is_whitespace();
        if at_token_start && chars[i..].starts_with(&pattern[..]) {
            let mut j = i + pattern.len();
            if chars.get(j) == Some(&'"') {
                j += 1;
                while j < chars.len() {
                    match chars[j] {
                        '\\' => j += 2,
                        '"' => {
                            j += 1;
                            break;
                        }
                        _ => j += 1,
                    }
                }
            } else {
                while j < chars.len() && !chars[j].is_whitespace() && chars[j] != '{' && chars[j] != '}' {
                    j += 1;
                }
            }
            return Some((i, j));
        }
        if c == '"' {
            in_string = true;
        }
        i += 1;
    }
    None
}

// Removes every `key=value` property from a KDL line.
fn remove_prop(line: &str, key: &str) -> String {
    let mut chars: Vec<char> = line.chars().collect();
    while let Some((start, end)) = prop_span(&chars, key) {
        let start = if chars[start - 1] == ' ' { start - 1 } else { start };
        chars.drain(start..end);
    }
    chars.into_iter().collect()
}

// The full `key=value` token of a property, verbatim.
fn prop_token(line: &str, key: &str) -> Option<String> {
    let chars: Vec<char> = line.chars().collect();
    prop_span(&chars, key).map(|(start, end)| chars[start..end].iter().collect())
}

// get_focused_pane_info reports the server's stable tab id; everything else
// in the plugin (own_tab, instance tabs) speaks display positions, so the
// id is translated through the TabUpdate-derived states. An id with no
// position yet (stale TabUpdate) falls back to the cached position.
fn active_tab_for_decision(
    cached_active_tab: Option<usize>,
    focused_pane_info: Result<(usize, PaneId), String>,
    tab_states: &BTreeMap<usize, TabState>,
) -> Option<usize> {
    focused_pane_info
        .ok()
        .and_then(|(tab_id, _)| {
            tab_states
                .iter()
                .find(|(_, state)| state.id == tab_id)
                .map(|(position, _)| *position)
        })
        .or(cached_active_tab)
}

fn sidebar_instances(manifest: &PaneManifest) -> Vec<SidebarInstance> {
    let mut instances: Vec<SidebarInstance> = manifest
        .panes
        .iter()
        .flat_map(|(tab, panes)| {
            panes
                .iter()
                .filter(|p| {
                    p.is_plugin
                        && p.plugin_url
                            .as_deref()
                            .is_some_and(|u| u.contains("zellij-sidebar"))
                })
                .map(move |p| SidebarInstance {
                    pane_id: p.id,
                    tab: *tab,
                    floating: p.is_floating,
                })
        })
        .collect();
    instances.sort_unstable_by_key(|i| i.pane_id);
    instances
}

fn move_selection(current: usize, delta: isize, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    (current as isize + delta).clamp(0, len as isize - 1) as usize
}

// Zellij mouse positions are pane-local and one-based. Each pane occupies two
// display lines (title + status) below the header.
fn target_for_line(line: isize, row_count: usize) -> LineTarget {
    if line <= 0 {
        return LineTarget::None;
    }
    if line == 1 {
        return LineTarget::Header;
    }
    let idx = (line as usize - 2) / 2;
    if idx < row_count {
        LineTarget::Row(idx)
    } else {
        LineTarget::None
    }
}

fn state_glyph(state: agent::AgentState) -> &'static str {
    match state {
        agent::AgentState::Blocked => "\u{1b}[31m\u{25cf}\u{1b}[0m ",
        agent::AgentState::Working => "\u{1b}[33m\u{25cf}\u{1b}[0m ",
        agent::AgentState::Done => "\u{1b}[36m\u{25cf}\u{1b}[0m ",
        agent::AgentState::Idle => "\u{1b}[32m\u{2713}\u{1b}[0m ",
        agent::AgentState::Unknown => "  ",
    }
}

fn state_marker(fields: &agent::AgentFields) -> &'static str {
    state_glyph(fields.state)
}

// First line of a session row: the session's state marker and agent name,
// with an explicit ·unbound tag when no listed pane matches its cwd.
fn session_row_line(session: &SessionEvent, bound: bool, cols: usize) -> String {
    let text = if bound {
        session.agent.clone()
    } else {
        format!("{} \u{b7}unbound", session.agent)
    };
    let text: String = text.chars().take(cols.saturating_sub(2)).collect();
    format!(
        "{}{}",
        state_glyph(agent::marker_for_state(&session.state)),
        text
    )
}

// First line of a gate row: a pending gate waits on a verdict, so it wears
// the blocked marker; the entity title names what is under review.
fn gate_row_line(gate: &GateEvent, cols: usize) -> String {
    let title: String = gate
        .entity_title
        .chars()
        .take(cols.saturating_sub(2))
        .collect();
    format!("{}{}", state_glyph(agent::AgentState::Blocked), title)
}

// Second line of a gate row: where the gate sits and what the reviewer
// recommends, sized like the pane rows' dim status line.
fn gate_row_detail(gate: &GateEvent, cols: usize) -> String {
    format!("{} r{} \u{b7} {}", gate.stage, gate.round, gate.recommendation)
        .chars()
        .take(cols.saturating_sub(4))
        .collect()
}

fn row_marker(row: &Row) -> &'static str {
    state_marker(&row.agent)
}

// Compact 1-glyph-per-row rendering below STATUS_MIN_COLS: no header text,
// no titles/status/summary — one state marker per row, session, and gate,
// in the same top-to-bottom order render() prints them in. Reuses the same
// glyph functions the normal-width render calls, so the compact mode never
// shows a color/symbol the normal mode wouldn't have shown for that item.
fn sliver_lines(rows: &[Row], sessions: &[SessionEvent], gates: &[GateEvent]) -> Vec<String> {
    rows.iter()
        .map(|row| row_marker(row).trim_end().to_owned())
        .chain(sessions.iter().map(|session| {
            state_glyph(agent::marker_for_state(&session.state)).trim_end().to_owned()
        }))
        .chain(gates.iter().map(|_| state_glyph(agent::AgentState::Blocked).trim_end().to_owned()))
        .collect()
}

fn title_style(row: &Row) -> &'static str {
    match (row.focused, row.agent.kind == agent::AgentKind::Unknown) {
        (true, false) => "\u{1b}[1;36m",
        (true, true) => "\u{1b}[1m",
        (false, false) => "\u{1b}[36m",
        (false, true) => "",
    }
}

fn preserve_agent_fields(rows: &mut [Row], old: &[Row]) {
    for row in rows.iter_mut() {
        if let Some(prev) = old.iter().find(|r| r.pane_id == row.pane_id) {
            row.agent = prev.agent.clone();
        }
    }
}

fn own_tab_position(manifest: &PaneManifest, own_plugin_id: u32) -> Option<usize> {
    manifest.panes.iter().find_map(|(tab, panes)| {
        panes
            .iter()
            .any(|p| p.is_plugin && p.id == own_plugin_id)
            .then_some(*tab)
    })
}

// Rows for the tab this plugin lives in: terminal panes only, top-to-bottom,
// excluding suppressed/unselectable panes. Agent identity is enriched by
// timer polling, after the manifest has created the base rows.
fn rows_for_own_tab(manifest: &PaneManifest, own_plugin_id: u32) -> Vec<Row> {
    let Some(tab) = own_tab_position(manifest, own_plugin_id) else {
        return Vec::new();
    };
    let mut panes: Vec<&PaneInfo> = manifest
        .panes
        .get(&tab)
        .map(|v| v.iter().collect())
        .unwrap_or_default();
    panes.retain(|p| !p.is_plugin && p.is_selectable && !p.is_suppressed);
    panes.sort_by_key(|p| (p.pane_y, p.pane_x));
    panes
        .iter()
        .map(|p| Row {
            pane_id: p.id,
            title: p.title.clone(),
            focused: p.is_focused,
            agent: agent::AgentFields::default(),
        })
        .collect()
}

// Host test builds cannot link the wasm host import; tests only exercise pure
// functions, so a no-op stub satisfies the linker.
#[cfg(all(test, not(target_family = "wasm")))]
#[no_mangle]
extern "C" fn host_run_plugin_command() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_pipe_permission_is_reserved_for_token_bound_entry() {
        let installed = BTreeMap::new();
        let mut empty_token = BTreeMap::new();
        empty_token.insert("recipient_token".to_owned(), String::new());
        let mut direct_entry = BTreeMap::new();
        direct_entry.insert("recipient_token".to_owned(), "entry-token".to_owned());
        let installed_permissions = vec![
            PermissionType::ReadApplicationState,
            PermissionType::ChangeApplicationState,
            PermissionType::ReadPaneContents,
            PermissionType::Reconfigure,
            PermissionType::RunCommands,
        ];

        assert_eq!(permissions_for_config(&installed), installed_permissions);
        assert_eq!(permissions_for_config(&empty_token), installed_permissions);
        assert_eq!(
            permissions_for_config(&direct_entry),
            vec![
                PermissionType::ReadApplicationState,
                PermissionType::ChangeApplicationState,
                PermissionType::ReadPaneContents,
                PermissionType::ReadCliPipes,
                PermissionType::Reconfigure,
                PermissionType::RunCommands,
            ]
        );
    }

    fn pane(id: u32, is_plugin: bool, title: &str, y: usize, focused: bool) -> PaneInfo {
        PaneInfo {
            id,
            is_plugin,
            title: title.to_owned(),
            pane_y: y,
            is_focused: focused,
            is_selectable: true,
            ..Default::default()
        }
    }

    fn sidebar_pane(id: u32, floating: bool) -> PaneInfo {
        let mut p = pane(id, true, "sidebar", 0, false);
        p.plugin_url = Some("file:/tmp/zellij-sidebar.wasm".to_owned());
        p.is_floating = floating;
        p
    }

    fn inst(pane_id: u32, tab: usize, floating: bool) -> SidebarInstance {
        SidebarInstance {
            pane_id,
            tab,
            floating,
        }
    }

    fn tab_info(
        position: usize,
        tab_id: usize,
        active: bool,
        swap_name: Option<&str>,
        swap_dirty: bool,
    ) -> TabInfo {
        TabInfo {
            position,
            tab_id,
            active,
            active_swap_layout_name: swap_name.map(str::to_owned),
            is_swap_layout_dirty: swap_dirty,
            ..Default::default()
        }
    }

    fn manifest(tabs: Vec<(usize, Vec<PaneInfo>)>) -> PaneManifest {
        let mut panes = std::collections::HashMap::new();
        for (tab, list) in tabs {
            panes.insert(tab, list);
        }
        PaneManifest { panes }
    }

    // The two pinned agent-event row kinds (plan decision 3,
    // docs/plan-agent-rail.md), transcribed with grout's exact field sets —
    // fixtures from outside this plugin's source.
    fn session_line() -> &'static str {
        r#"{"kind":"session","id":"01J9SESS","pane_id":4,"cwd":"/Users/clkao/git/zaphod","agent":"claude","state":"working","summary":"wiring the rows section","ts":"2026-07-07T05:00:00Z"}"#
    }

    fn gate_line() -> &'static str {
        r#"{"kind":"gate","log_path":"/pg/brief.decisions.jsonl","workflow":"agent-rail-dev","entity":"plugin-rows-section","entity_title":"Plugin rows section","stage":"ideation","round":2,"recommendation":"APPROVED","ts":"2026-07-07T05:00:00Z"}"#
    }

    #[test]
    fn parses_both_pinned_row_kinds() {
        assert_eq!(
            parse_agent_event(session_line()).unwrap(),
            AgentEvent::Session(SessionEvent {
                id: "01J9SESS".to_owned(),
                pane_id: Some(4),
                cwd: "/Users/clkao/git/zaphod".to_owned(),
                agent: "claude".to_owned(),
                state: "working".to_owned(),
                summary: "wiring the rows section".to_owned(),
            })
        );
        assert_eq!(
            parse_agent_event(gate_line()).unwrap(),
            AgentEvent::Gate(GateEvent {
                log_path: "/pg/brief.decisions.jsonl".to_owned(),
                entity_title: "Plugin rows section".to_owned(),
                stage: "ideation".to_owned(),
                round: 2,
                recommendation: "APPROVED".to_owned(),
            })
        );
    }

    #[test]
    fn tolerates_missing_fields_and_unknown_extras() {
        // grout pins exact field sets today; tolerance is the plugin's
        // concern: extra fields are ignored, missing declared fields default
        // (a session without cwd simply renders unbound).
        let event =
            parse_agent_event(r#"{"kind":"session","id":"s1","next_sprint_field":true}"#).unwrap();
        assert_eq!(
            event,
            AgentEvent::Session(SessionEvent {
                id: "s1".to_owned(),
                ..Default::default()
            })
        );
    }

    #[test]
    fn rejects_unknown_kind_and_malformed_json() {
        let unknown = parse_agent_event(r#"{"kind":"deploy","id":"x"}"#).unwrap_err();
        assert!(
            unknown.contains("deploy"),
            "reason names the unknown kind: {unknown}"
        );
        assert!(!parse_agent_event("{not json").unwrap_err().is_empty());
        // A payload without a kind tag is an error, never a default kind.
        assert!(parse_agent_event(r#"{"id":"x"}"#).is_err());
    }

    #[test]
    fn session_upsert_replaces_by_id() {
        let mut sessions = Vec::new();
        let mut gates = Vec::new();
        let first = parse_agent_event(session_line()).unwrap();
        assert!(apply_agent_event(&mut sessions, &mut gates, first.clone()));
        // Re-applying the identical line changes nothing: no re-render.
        assert!(!apply_agent_event(&mut sessions, &mut gates, first));
        // The same id with updated fields replaces the row, never duplicates.
        let updated = parse_agent_event(&session_line().replace("working", "done")).unwrap();
        assert!(apply_agent_event(&mut sessions, &mut gates, updated));
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].state, "done");
        assert!(gates.is_empty());
    }

    #[test]
    fn gate_upsert_replaces_by_log_path_keeping_insertion_order() {
        let mut sessions = Vec::new();
        let mut gates = Vec::new();
        let first = GateEvent {
            log_path: "/a.decisions.jsonl".to_owned(),
            round: 1,
            ..Default::default()
        };
        let second = GateEvent {
            log_path: "/b.decisions.jsonl".to_owned(),
            round: 1,
            ..Default::default()
        };
        assert!(apply_agent_event(&mut sessions, &mut gates, AgentEvent::Gate(first.clone())));
        assert!(apply_agent_event(&mut sessions, &mut gates, AgentEvent::Gate(second)));
        // A later round for the first gate updates it in place.
        let rerun = GateEvent { round: 2, ..first };
        assert!(apply_agent_event(&mut sessions, &mut gates, AgentEvent::Gate(rerun)));
        assert_eq!(
            gates.iter().map(|g| g.log_path.as_str()).collect::<Vec<_>>(),
            vec!["/a.decisions.jsonl", "/b.decisions.jsonl"]
        );
        assert_eq!(gates[0].round, 2);
        assert!(sessions.is_empty());
    }

    fn cwd_row(pane_id: u32) -> Row {
        Row {
            pane_id,
            ..Default::default()
        }
    }

    fn cwd_map(entries: &[(u32, &str)]) -> BTreeMap<u32, std::path::PathBuf> {
        entries
            .iter()
            .map(|(id, cwd)| (*id, std::path::PathBuf::from(cwd)))
            .collect()
    }

    #[test]
    fn registered_pane_binds_despite_same_cwd_lookalikes() {
        let rows = [cwd_row(4), cwd_row(8)];
        assert_eq!(
			registered_session_pane(&SessionEvent {
				pane_id: Some(4),
				cwd: "/deliberately/wrong".to_owned(),
				..Default::default()
			}, &rows),
            Some(4)
        );
		let same_cwd = cwd_map(&[(4, "/same"), (8, "/same")]);
		assert_eq!(same_cwd.len(), 2);
		assert_eq!(registered_session_pane(&SessionEvent { pane_id: Some(8), cwd: "/same".to_owned(), ..Default::default() }, &rows), Some(8));
    }

    #[test]
    fn absent_or_stale_registered_pane_renders_unbound() {
        let rows = [cwd_row(4)];
		assert_eq!(registered_session_pane(&SessionEvent::default(), &rows), None);
		assert_eq!(registered_session_pane(&SessionEvent { pane_id: Some(99), cwd: "/same".to_owned(), ..Default::default() }, &rows), None);
    }

    #[test]
    fn brief_path_inverts_the_decision_log_suffix() {
        assert_eq!(
            brief_path_for_log("/pg/brief.decisions.jsonl"),
            Some("/pg/brief.md".to_owned())
        );
        // A log path without the suffix yields no brief: the row still
        // renders, but its click must never float a wrong file.
        assert_eq!(brief_path_for_log("/pg/brief.jsonl"), None);
        assert_eq!(brief_path_for_log(""), None);
    }

    #[test]
    fn sectioned_lines_map_headers_rows_and_beyond_for_p2_s1_g1() {
        // Hand-counted against the render order: PANES header, two 2-line
        // pane rows, AGENTS header, one 2-line session row, GATES header,
        // one 2-line gate row. Section headers are not controls.
        let layout = section_layout(2, 1, 1);
        assert_eq!(layout.target(0), LineTarget::None);
        assert_eq!(layout.target(1), LineTarget::Header);
        assert_eq!(layout.target(2), LineTarget::Row(0));
        assert_eq!(layout.target(5), LineTarget::Row(1));
        assert_eq!(layout.target(6), LineTarget::None); // AGENTS header
        assert_eq!(layout.target(7), LineTarget::SessionRow(0));
        assert_eq!(layout.target(8), LineTarget::SessionRow(0));
        assert_eq!(layout.target(9), LineTarget::None); // GATES header
        assert_eq!(layout.target(10), LineTarget::GateRow(0));
        assert_eq!(layout.target(11), LineTarget::GateRow(0));
        assert_eq!(layout.target(12), LineTarget::None);
    }

    #[test]
    fn empty_sections_map_like_the_pane_only_rail() {
        // Zero footprint: with no agent events received the sectioned map is
        // the shipped pane-rows map on every line.
        let layout = section_layout(2, 0, 0);
        for line in -1..10 {
            assert_eq!(layout.target(line), target_for_line(line, 2));
        }
    }

    #[test]
    fn gates_section_starts_right_after_pane_rows_when_no_sessions_arrived() {
        let layout = section_layout(1, 0, 2);
        assert_eq!(layout.target(3), LineTarget::Row(0));
        assert_eq!(layout.target(4), LineTarget::None); // GATES header
        assert_eq!(layout.target(5), LineTarget::GateRow(0));
        assert_eq!(layout.target(7), LineTarget::GateRow(1));
        assert_eq!(layout.target(9), LineTarget::None);
    }

    #[test]
    fn session_row_click_focuses_only_when_bound() {
        let rows = vec![cwd_row(4), cwd_row(8)];
        let cwds = cwd_map(&[(4, "/Users/clkao/git/zaphod"), (8, "/tmp")]);
        let sessions = vec![SessionEvent {
            pane_id: Some(4),
            cwd: "/Users/clkao/git/zaphod".to_owned(),
            ..Default::default()
        }];
        let gates = Vec::new();
        // P=2,S=1: the session row occupies lines 7-8.
        assert_eq!(
            decide_rail_click(7, &rows, &sessions, &gates, &cwds),
            ClickAction::FocusPane(4)
        );
        // An unbound session row's click is dead — never guessed.
        let unmatched = vec![SessionEvent {
            cwd: "/nowhere".to_owned(),
            ..Default::default()
        }];
        assert_eq!(
            decide_rail_click(8, &rows, &unmatched, &gates, &cwds),
            ClickAction::None
        );
    }

    #[test]
    fn gate_click_floats_tui_on_brief() {
        let rows = vec![cwd_row(4), cwd_row(8)];
        let cwds = BTreeMap::new();
        let sessions = vec![SessionEvent::default()];
        let gates = vec![GateEvent {
            log_path: "/pg/brief.decisions.jsonl".to_owned(),
            ..Default::default()
        }];
        // P=2,S=1,G=1: the gate row occupies lines 10-11.
        assert_eq!(
            decide_rail_click(10, &rows, &sessions, &gates, &cwds),
            ClickAction::FloatGate {
                brief: "/pg/brief.md".to_owned(),
                log: "/pg/brief.decisions.jsonl".to_owned(),
            }
        );
        // The pane region still routes through the shipped decider.
        assert_eq!(
            decide_rail_click(1, &rows, &sessions, &gates, &cwds),
            ClickAction::ToggleDock
        );
        assert_eq!(
            decide_rail_click(2, &rows, &sessions, &gates, &cwds),
            ClickAction::FocusPane(4)
        );
    }

    #[test]
    fn bad_log_suffix_never_floats() {
        // The row renders, but a log_path that does not end .decisions.jsonl
        // has no derivable brief: clicking it must do nothing.
        let gates = vec![GateEvent {
            log_path: "/pg/notes.txt".to_owned(),
            ..Default::default()
        }];
        // P=0,S=0,G=1: GATES header at line 2, gate row lines 3-4.
        assert_eq!(
            decide_rail_click(3, &[], &[], &gates, &BTreeMap::new()),
            ClickAction::None
        );
    }

    fn agent_event(payload: Option<&str>) -> PipeMessage {
        PipeMessage {
            source: PipeSource::Keybind,
            name: "agent-event".to_owned(),
            payload: payload.map(str::to_owned),
            args: BTreeMap::new(),
            is_private: false,
        }
    }

    fn agent_event_for_tab(payload: &str, recipient_tab_id: &str) -> PipeMessage {
        let mut message = agent_event(Some(payload));
        message.name = "zaphod-agent-v1-test-token-event".to_owned();
        message
            .args
            .insert("recipient-tab-id".to_owned(), recipient_tab_id.to_owned());
        message
            .args
            .insert("recipient-token".to_owned(), "test-token".to_owned());
        message
    }

    fn agent_snapshot(payload: Option<&str>, recipient_tab_id: &str) -> PipeMessage {
        let mut message = agent_event(payload);
        message.name = "zaphod-agent-v1-test-token-snapshot".to_owned();
        message
            .args
            .insert("recipient-tab-id".to_owned(), recipient_tab_id.to_owned());
        message
            .args
            .insert("recipient-token".to_owned(), "test-token".to_owned());
        message
    }

    fn arm_agent_recipient(sidebar: &mut Sidebar, own_position: usize, tabs: &[TabInfo]) {
		sidebar.config.insert("recipient_token".to_owned(), "test-token".to_owned());
        sidebar.own_tab = Some(own_position);
        sidebar.own_floating = false;
        sidebar.observe_agent_manifest();
        sidebar.observe_agent_tab_update(tabs);
    }

    #[test]
    fn agent_event_requires_a_fresh_unique_stable_tab_recipient() {
        let tabs = [
            tab_info(1, 0, true, None, false),
            tab_info(2, 81, false, None, false),
        ];
        let mut target = Sidebar::default();
        arm_agent_recipient(&mut target, 1, &tabs);

        // Server tab ID zero is a real identity, not the default/unavailable
        // value. The target accepts exactly its canonical decimal recipient.
        assert!(target.pipe(agent_event_for_tab(session_line(), "0")));
        assert_eq!(target.sessions.len(), 1);
        target.rows = vec![cwd_row(4)];
        target.pane_cwds = cwd_map(&[(4, "/Users/clkao/git/zaphod")]);
        // P=1,S=1: the accepted session's first render line is 5. CWD only
        // resolves focus after stable-tab admission has selected this rail.
        assert_eq!(
            decide_rail_click(
                5,
                &target.rows,
                &target.sessions,
                &target.gates,
                &target.pane_cwds
            ),
            ClickAction::FocusPane(4)
        );

        let mut bystander = Sidebar::default();
        arm_agent_recipient(&mut bystander, 2, &tabs);
        assert!(!bystander.pipe(agent_event_for_tab(session_line(), "0")));
        assert!(bystander.sessions.is_empty());
        assert_eq!(
            decide_rail_click(
                5,
                &[cwd_row(4)],
                &bystander.sessions,
                &bystander.gates,
                &cwd_map(&[(4, "/Users/clkao/git/zaphod")])
            ),
            ClickAction::None
        );

        let mut missing = Sidebar::default();
        arm_agent_recipient(&mut missing, 1, &tabs);
        assert!(!missing.pipe(agent_event(Some(session_line()))));
        assert!(missing.sessions.is_empty());

        for bad_recipient in ["", "00", "01", "+0", " 0", "0 ", "-0", "nope", "82"] {
            let mut rail = Sidebar::default();
            arm_agent_recipient(&mut rail, 1, &tabs);
            assert!(
                !rail.pipe(agent_event_for_tab(session_line(), bad_recipient)),
                "recipient {bad_recipient:?} must be inert"
            );
            assert!(rail.sessions.is_empty());
        }

        // A new PaneUpdate invalidates the mapping until a later TabUpdate
        // proves the current manifest position again.
        let mut stale = Sidebar::default();
        arm_agent_recipient(&mut stale, 1, &tabs);
        stale.observe_agent_manifest();
        assert!(!stale.pipe(agent_event_for_tab(session_line(), "0")));
        assert!(stale.sessions.is_empty());

        // A duplicate stable ID is ambiguous even when one matching display
        // position exists, so neither rail may apply the broadcast.
        let duplicate = [
            tab_info(1, 0, true, None, false),
            tab_info(2, 0, false, None, false),
        ];
        let mut ambiguous = Sidebar::default();
        arm_agent_recipient(&mut ambiguous, 1, &duplicate);
        assert!(!ambiguous.pipe(agent_event_for_tab(session_line(), "0")));
        assert!(ambiguous.sessions.is_empty());

        let mut unavailable = Sidebar::default();
        unavailable.own_tab = Some(1);
        unavailable.own_floating = false;
        unavailable.observe_agent_manifest();
        unavailable.observe_agent_tab_update(&[tab_info(2, 81, false, None, false)]);
        assert!(!unavailable.pipe(agent_event_for_tab(session_line(), "0")));
        assert!(unavailable.sessions.is_empty());

        let mut floating = Sidebar::default();
        arm_agent_recipient(&mut floating, 1, &tabs);
        floating.own_floating = true;
        assert!(!floating.pipe(agent_event_for_tab(session_line(), "0")));
        assert!(floating.sessions.is_empty());
    }

    #[test]
    fn legacy_agent_event_name_is_inert_even_for_exact_recipient() {
        let mut sidebar = Sidebar::default();
        arm_agent_recipient(&mut sidebar, 1, &[tab_info(1, 73, true, None, false)]);
        let mut legacy = agent_event(Some(session_line()));
        legacy
            .args
            .insert("recipient-tab-id".to_owned(), "73".to_owned());
        legacy
            .args
            .insert("recipient-token".to_owned(), "test-token".to_owned());

        assert!(!sidebar.pipe(legacy));
        assert!(sidebar.sessions.is_empty());
    }

    #[test]
    fn agent_snapshot_requires_valid_payload_and_exact_recipient() {
        let tabs = [
            tab_info(1, 73, true, None, false),
            tab_info(2, 81, false, None, false),
        ];
        let payload = format!("[{},{}]", session_line(), gate_line());
        let mut target = Sidebar::default();
        arm_agent_recipient(&mut target, 1, &tabs);
        assert!(target.pipe(agent_snapshot(Some(&payload), "73")));
        assert_eq!(target.sessions.len(), 1);
        assert_eq!(target.gates.len(), 1);

        for invalid in [None, Some("{not json"), Some(r#"{"kind":"session"}"#)] {
            let mut rail = Sidebar::default();
            arm_agent_recipient(&mut rail, 1, &tabs);
            assert!(!rail.pipe(agent_snapshot(invalid, "73")));
            assert!(rail.sessions.is_empty());
            assert!(rail.gates.is_empty());
        }

        let mut foreign = Sidebar::default();
        arm_agent_recipient(&mut foreign, 2, &tabs);
        assert!(!foreign.pipe(agent_snapshot(Some(&payload), "73")));
        assert!(foreign.sessions.is_empty());
        assert!(foreign.gates.is_empty());

        let mut same_tab_competitor = Sidebar::default();
        arm_agent_recipient(&mut same_tab_competitor, 1, &tabs);
        same_tab_competitor
            .config
            .insert("recipient_token".to_owned(), "other-token".to_owned());
        assert!(!same_tab_competitor.pipe(agent_snapshot(Some(&payload), "73")));
        assert!(same_tab_competitor.sessions.is_empty());
    }

    #[test]
    fn session_snapshot_replaces_stale_rows_and_rejects_duplicate_pane_authority() {
        let mut sessions = vec![SessionEvent {
            id: "stale".to_owned(),
            pane_id: Some(9),
            ..Default::default()
        }];
        let mut gates = vec![GateEvent {
            log_path: "/gate.decisions.jsonl".to_owned(),
            ..Default::default()
        }];
        assert!(apply_agent_snapshot(&mut sessions, &mut gates, Some("[]")).unwrap());
        assert!(sessions.is_empty(), "empty authoritative snapshot removes stale rows");
        assert_eq!(gates.len(), 1, "session snapshots do not erase gate state");

        let duplicate = r#"[
          {"kind":"session","id":"one","pane_id":7},
          {"kind":"session","id":"two","pane_id":7}
        ]"#;
        let before = sessions.clone();
        assert!(apply_agent_snapshot(&mut sessions, &mut gates, Some(duplicate)).is_err());
        assert_eq!(sessions, before, "invalid snapshot is atomic");
    }

    #[test]
    fn agent_event_lines_land_as_session_and_gate_rows() {
        let mut sidebar = Sidebar::default();
        arm_agent_recipient(&mut sidebar, 1, &[tab_info(1, 73, true, None, false)]);
        assert!(
            sidebar.pipe(agent_event_for_tab(session_line(), "73")),
            "a new row re-renders"
        );
        assert!(sidebar.pipe(agent_event_for_tab(gate_line(), "73")));
        assert_eq!(sidebar.sessions.len(), 1);
        assert_eq!(sidebar.sessions[0].agent, "claude");
        assert_eq!(sidebar.gates.len(), 1);
        assert_eq!(sidebar.gates[0].round, 2);
        // The identical line again changes nothing: no re-render.
        assert!(!sidebar.pipe(agent_event_for_tab(session_line(), "73")));
        assert_eq!(sidebar.sessions.len(), 1);
    }

    #[test]
    fn unknown_kind_dropped() {
        let mut sidebar = Sidebar::default();
        assert!(!sidebar.pipe(agent_event(Some(r#"{"kind":"deploy","id":"x"}"#))));
        assert!(sidebar.sessions.is_empty() && sidebar.gates.is_empty());
    }

    #[test]
    fn malformed_json_dropped() {
        let mut sidebar = Sidebar::default();
        assert!(!sidebar.pipe(agent_event(Some("{not json"))));
        assert!(sidebar.sessions.is_empty() && sidebar.gates.is_empty());
    }

    #[test]
    fn missing_payload_dropped() {
        let mut sidebar = Sidebar::default();
        assert!(!sidebar.pipe(agent_event(None)));
        assert!(sidebar.sessions.is_empty() && sidebar.gates.is_empty());
    }

    #[test]
    fn pinned_protocol_lines_render_and_bind() {
        // Wire to action: the two pinned lines plus a pane set whose one
        // matching cwd equals the session's cwd yield a bound session row
        // and an actionable gate row.
        let mut sidebar = Sidebar::default();
        arm_agent_recipient(&mut sidebar, 1, &[tab_info(1, 73, true, None, false)]);
        assert!(sidebar.pipe(agent_event_for_tab(session_line(), "73")));
        assert!(sidebar.pipe(agent_event_for_tab(gate_line(), "73")));
        sidebar.rows = vec![cwd_row(4), cwd_row(8)];
        sidebar.pane_cwds = cwd_map(&[(4, "/Users/clkao/git/zaphod"), (8, "/tmp")]);
        // The session row's marker reflects the line's state; bound rows
        // carry no unbound tag and click through to the cwd-bound pane.
        let session = &sidebar.sessions[0];
        let bound = registered_session_pane(session, &sidebar.rows);
        assert_eq!(bound, Some(4));
        let line = session_row_line(session, bound.is_some(), 28);
        assert!(line.starts_with(state_glyph(agent::AgentState::Working)));
        assert!(line.contains("claude"));
        assert!(!line.contains("\u{b7}unbound"));
        assert_eq!(
            decide_rail_click(7, &sidebar.rows, &sidebar.sessions, &sidebar.gates, &sidebar.pane_cwds),
            ClickAction::FocusPane(4)
        );
        // The gate row names the entity under review and its stage detail;
        // its click floats the TUI on the brief with the verbatim log path.
        let gate = &sidebar.gates[0];
        assert!(gate_row_line(gate, 28).contains("Plugin rows section"));
        assert_eq!(gate_row_detail(gate, 80), "ideation r2 \u{b7} APPROVED");
        assert_eq!(
            decide_rail_click(10, &sidebar.rows, &sidebar.sessions, &sidebar.gates, &sidebar.pane_cwds),
            ClickAction::FloatGate {
                brief: "/pg/brief.md".to_owned(),
                log: "/pg/brief.decisions.jsonl".to_owned(),
            }
        );
    }

    #[test]
    fn unbound_session_rows_carry_the_unbound_tag() {
        let session = SessionEvent {
            agent: "claude".to_owned(),
            state: "blocked".to_owned(),
            ..Default::default()
        };
        let line = session_row_line(&session, false, 28);
        assert!(line.starts_with(state_glyph(agent::AgentState::Blocked)));
        assert!(line.ends_with("claude \u{b7}unbound"));
        // Row text truncates to the rail width like pane rows.
        let narrow = session_row_line(&session, false, 8);
        assert!(narrow.ends_with("claude"));
        assert!(!narrow.contains("\u{b7}unbound"));
    }

    #[test]
    fn trace_is_gated_off_unless_config_sets_a_nonempty_debug_value() {
        assert!(!debug_enabled(&BTreeMap::new()), "absent key stays silent");
        let mut config = BTreeMap::new();
        config.insert("debug".to_owned(), String::new());
        assert!(!debug_enabled(&config), "empty value stays silent");
        config.insert("debug".to_owned(), "1".to_owned());
        assert!(debug_enabled(&config), "non-empty value enables tracing");
    }

    #[test]
    fn wedge_drill_runs_only_with_a_numeric_wedge_poll_secs_value() {
        assert_eq!(wedge_poll_secs(&BTreeMap::new()), None, "absent key: no drill");
        let mut config = BTreeMap::new();
        config.insert("wedge_poll_secs".to_owned(), "15".to_owned());
        assert_eq!(wedge_poll_secs(&config), Some(15));
        config.insert("wedge_poll_secs".to_owned(), String::new());
        assert_eq!(wedge_poll_secs(&config), None, "empty value: no drill");
        config.insert("wedge_poll_secs".to_owned(), "slow".to_owned());
        assert_eq!(wedge_poll_secs(&config), None, "non-numeric value: no drill");
    }

    #[test]
    fn steer_is_armed_only_for_the_actors_own_tab() {
        assert!(
            steer_completes_locally(Some(2), 2),
            "an actor rebuilding the tab it lives in arms the completing steer"
        );
        assert!(
            !steer_completes_locally(Some(1), 2),
            "a remote actor rebuilding a tab it does not live in arms no steer"
        );
        assert!(
            !steer_completes_locally(None, 2),
            "an actor whose own tab is unknown arms no steer"
        );
    }

    #[test]
    fn polls_only_the_active_tabs_docked_rail() {
        // The active tab's docked rail is the only on-screen status; poll it.
        assert!(should_poll_statuses(Some(1), Some(1), DOCKED_COLS));
        // A background tab's rail is off screen — skip.
        assert!(!should_poll_statuses(Some(1), Some(2), DOCKED_COLS));
        // The undocked 1-col sliver shows nothing — skip even when active.
        assert!(!should_poll_statuses(Some(1), Some(1), UNDOCKED_COLS));
        // Unknown active tab: bias toward polling (a skip only leaves the
        // previous status on screen, never a blank).
        assert!(should_poll_statuses(Some(1), None, DOCKED_COLS));
        assert!(should_poll_statuses(None, None, DOCKED_COLS));
    }

    #[test]
    fn poll_backoff_skips_grow_and_cap() {
        assert_eq!(backoff_skips(0), 0);
        assert_eq!(backoff_skips(1), 1);
        assert_eq!(backoff_skips(2), 3);
        assert_eq!(backoff_skips(3), 7);
        assert_eq!(backoff_skips(4), 15);
        assert_eq!(backoff_skips(20), 15); // capped, no overflow
    }

    #[test]
    fn wedge_classified_call_aborts_the_pass() {
        // The observed wild wedge: ~13s per GetPaneRunningCommand call.
        assert!(wedge_aborts_pass(Duration::from_secs(13)));
        // The classification threshold itself.
        assert!(wedge_aborts_pass(Duration::from_secs(1)));
    }

    #[test]
    fn fast_call_continues_the_pass() {
        // Healthy calls return well inside the 2s poll timer.
        assert!(!wedge_aborts_pass(Duration::from_millis(50)));
        assert!(!wedge_aborts_pass(Duration::from_millis(999)));
    }

    #[test]
    fn wedge_outcome_lands_in_the_panes_backoff_and_a_success_clears_it() {
        let mut bo = PollBackoff::default();
        bo.record(true); // a wedge is recorded as a failure even when the call returned Ok
        assert_eq!(bo.skip, backoff_skips(1), "wedge grows the backoff");
        bo.record(true);
        assert_eq!(bo.skip, backoff_skips(2), "repeat wedges keep growing it");
        bo.record(false); // a later success clears it
        assert_eq!(bo.skip, 0);
        assert_eq!(bo.failures, 0);
    }

    #[test]
    fn aborted_pass_leaves_untried_panes_backoff_untouched() {
        // Drives refresh_statuses itself through the wedge branch: the drill
        // knob sleeps 1s in place of the host call, which wedge-classifies
        // every status call without reaching get_pane_scrollback.
        let mut sidebar = Sidebar::default();
        sidebar.wedge_poll_secs = Some(1);
        sidebar.rows = (1u32..=3)
            .map(|id| Row {
                pane_id: id,
                ..Default::default()
            })
            .collect();

        sidebar.refresh_statuses(); // pass 1: pane 1 wedges, the pass aborts
        let failures = |s: &Sidebar, id: u32| s.poll_backoff.get(&id).map(|b| b.failures);
        assert_eq!(failures(&sidebar, 1), Some(1), "the wedge lands in the pane's backoff");
        assert_eq!(failures(&sidebar, 2), None, "untried panes get no backoff entry");
        assert_eq!(failures(&sidebar, 3), None);

        sidebar.refresh_statuses(); // pass 2: pane 1 backed off, pane 2 wedges
        assert_eq!(failures(&sidebar, 1), Some(1), "the backed-off pane is skipped, not re-recorded");
        assert_eq!(failures(&sidebar, 2), Some(1), "the pass moves to the next due pane");
        assert_eq!(failures(&sidebar, 3), None, "the abort spares the panes behind the wedge");
    }

    #[test]
    fn cwd_map_prunes_to_live_rows() {
        // A closed pane's polled cwd must not linger in the map. The drill
        // knob stands in for the host calls: the pass aborts at the first
        // wedge-classified status call, before any cwd call, so the prune is
        // observable hermetically.
        let mut sidebar = Sidebar::default();
        sidebar.wedge_poll_secs = Some(1);
        sidebar.pane_cwds = cwd_map(&[(1, "/a"), (99, "/gone")]);
        sidebar.rows = vec![cwd_row(1)];
        sidebar.refresh_statuses();
        assert_eq!(sidebar.pane_cwds, cwd_map(&[(1, "/a")]));
    }

    #[test]
    fn a_repeatedly_failing_pane_backs_off_then_recovers() {
        let mut bo = PollBackoff::default();
        assert!(bo.due(), "first timer polls");
        bo.record(true); // timeout/not-found → one skip
        assert!(!bo.due(), "backed off one timer");
        assert!(bo.due(), "then polls again");
        bo.record(true); // second failure → three skips
        assert!(!bo.due());
        assert!(!bo.due());
        assert!(!bo.due());
        assert!(bo.due());
        bo.record(false); // a success resets the backoff
        assert!(bo.due(), "recovered: polls every timer");
        assert!(bo.due());
    }

    #[test]
    fn lists_only_terminal_panes_of_own_tab_sorted_top_to_bottom() {
        let m = manifest(vec![
            (0, vec![pane(9, false, "other-tab", 0, false)]),
            (
                1,
                vec![
                    pane(7, true, "sidebar", 0, false), // self (plugin id 7)
                    pane(3, false, "lower", 10, false),
                    pane(2, false, "upper", 2, true),
                ],
            ),
        ]);
        let rows = rows_for_own_tab(&m, 7);
        assert_eq!(
            rows.iter().map(|r| r.pane_id).collect::<Vec<_>>(),
            vec![2, 3]
        );
        assert!(rows[0].focused);
        assert!(!rows[1].focused);
        assert_eq!(rows[0].agent, agent::AgentFields::default());
    }

    #[test]
    fn initial_rows_start_with_unknown_agent_fields() {
        let m = manifest(vec![(
            0,
            vec![
                pane(7, true, "sidebar", 0, false),
                pane(1, false, "shell", 1, false),
            ],
        )]);
        let rows = rows_for_own_tab(&m, 7);
        assert_eq!(rows[0].agent, agent::AgentFields::default());
    }

    #[test]
    fn preserves_agent_fields_by_pane_id_after_manifest_rebuild() {
        let enriched = agent::AgentFields {
            kind: agent::AgentKind::Codex,
            state: agent::AgentState::Blocked,
            status: "allow command?".to_owned(),
            running_command: Some(vec!["codex".to_owned()]),
        };
        let old = vec![Row {
            pane_id: 1,
            title: "old".to_owned(),
            focused: false,
            agent: enriched.clone(),
        }];
        let mut rebuilt = vec![Row {
            pane_id: 1,
            title: "new".to_owned(),
            focused: true,
            agent: agent::AgentFields::default(),
        }];

        preserve_agent_fields(&mut rebuilt, &old);

        assert_eq!(rebuilt[0].agent, enriched);
        assert_eq!(rebuilt[0].title, "new");
        assert!(rebuilt[0].focused);
    }

    #[test]
    fn focused_rows_still_use_agent_state_marker() {
        let row = Row {
            pane_id: 1,
            title: "codex".to_owned(),
            focused: true,
            agent: agent::AgentFields {
                kind: agent::AgentKind::Codex,
                state: agent::AgentState::Blocked,
                status: "press enter to confirm".to_owned(),
                running_command: Some(vec!["codex".to_owned()]),
            },
        };

        assert_eq!(row_marker(&row), state_marker(&row.agent));
    }

    #[test]
    fn sliver_lines_carry_no_header_or_free_text() {
        let row = Row {
            pane_id: 1,
            title: "distinctive-pane-title".to_owned(),
            focused: false,
            agent: agent::AgentFields {
                kind: agent::AgentKind::Claude,
                state: agent::AgentState::Working,
                status: "distinctive-pane-status".to_owned(),
                running_command: None,
            },
        };
        let session = SessionEvent {
            agent: "distinctive-session-agent".to_owned(),
            state: "blocked".to_owned(),
            summary: "distinctive-session-summary".to_owned(),
            ..Default::default()
        };
        let gate = GateEvent {
            entity_title: "distinctive-gate-title".to_owned(),
            stage: "distinctive-gate-stage".to_owned(),
            recommendation: "distinctive-gate-recommendation".to_owned(),
            ..Default::default()
        };

        let lines = sliver_lines(
            std::slice::from_ref(&row),
            std::slice::from_ref(&session),
            std::slice::from_ref(&gate),
        );

        let joined = lines.join("\n");
        for banned in [
            "PANES",
            "AGENTS",
            "GATES",
            "\u{21c4}",
            "distinctive-pane-title",
            "distinctive-pane-status",
            "distinctive-session-agent",
            "distinctive-session-summary",
            "distinctive-gate-title",
            "distinctive-gate-stage",
            "distinctive-gate-recommendation",
        ] {
            assert!(
                !joined.contains(banned),
                "sliver output leaked {banned:?}: {joined:?}"
            );
        }

        assert_eq!(
            lines,
            vec![
                row_marker(&row).trim_end().to_owned(),
                state_glyph(agent::marker_for_state(&session.state))
                    .trim_end()
                    .to_owned(),
                state_glyph(agent::AgentState::Blocked).trim_end().to_owned(),
            ]
        );
    }

    #[test]
    fn sliver_lines_differ_when_one_row_state_changes() {
        let idle_row = |pane_id: u32| Row {
            pane_id,
            agent: agent::AgentFields {
                state: agent::AgentState::Idle,
                ..agent::AgentFields::default()
            },
            ..Default::default()
        };
        let all_idle = vec![idle_row(1), idle_row(2), idle_row(3)];
        let mut one_blocked = all_idle.clone();
        one_blocked[1].agent.state = agent::AgentState::Blocked;

        let idle_lines = sliver_lines(&all_idle, &[], &[]);
        let blocked_lines = sliver_lines(&one_blocked, &[], &[]);

        assert_ne!(idle_lines, blocked_lines);
        assert_eq!(
            blocked_lines[1],
            state_glyph(agent::AgentState::Blocked).trim_end()
        );
        assert_eq!(idle_lines[0], blocked_lines[0]);
        assert_eq!(idle_lines[2], blocked_lines[2]);
    }

    #[test]
    fn focused_known_agent_rows_have_distinct_title_style() {
        let focused = Row {
            pane_id: 1,
            title: "codex".to_owned(),
            focused: true,
            agent: agent::AgentFields {
                kind: agent::AgentKind::Codex,
                state: agent::AgentState::Working,
                status: "Esc to cancel".to_owned(),
                running_command: Some(vec!["codex".to_owned()]),
            },
        };
        let unfocused = Row {
            focused: false,
            ..focused.clone()
        };

        assert_ne!(title_style(&focused), title_style(&unfocused));
        assert_eq!(title_style(&focused), "\u{1b}[1;36m");
        assert_eq!(title_style(&unfocused), "\u{1b}[36m");
    }

    #[test]
    fn skips_suppressed_and_unselectable_panes() {
        let mut hidden = pane(4, false, "hidden", 3, false);
        hidden.is_suppressed = true;
        let mut unselectable = pane(5, false, "nope", 4, false);
        unselectable.is_selectable = false;
        let m = manifest(vec![(
            0,
            vec![pane(7, true, "sidebar", 0, false), hidden, unselectable],
        )]);
        assert!(rows_for_own_tab(&m, 7).is_empty());
    }

    #[test]
    fn empty_when_own_tab_not_found() {
        let m = manifest(vec![(0, vec![pane(1, false, "shell", 0, false)])]);
        assert!(rows_for_own_tab(&m, 99).is_empty());
    }

    #[test]
    fn maps_display_lines_to_header_and_two_line_rows() {
        assert_eq!(target_for_line(0, 2), LineTarget::None);
        assert_eq!(target_for_line(1, 2), LineTarget::Header);
        assert_eq!(target_for_line(2, 2), LineTarget::Row(0));
        assert_eq!(target_for_line(3, 2), LineTarget::Row(0));
        assert_eq!(target_for_line(4, 2), LineTarget::Row(1));
        assert_eq!(target_for_line(5, 2), LineTarget::Row(1));
        assert_eq!(target_for_line(6, 2), LineTarget::None);
        assert_eq!(target_for_line(-3, 2), LineTarget::None);
    }

    #[test]
    fn selection_moves_within_bounds() {
        assert_eq!(move_selection(0, -1, 5), 0);
        assert_eq!(move_selection(0, 1, 5), 1);
        assert_eq!(move_selection(4, 1, 5), 4);
        assert_eq!(move_selection(2, -1, 5), 1);
        assert_eq!(move_selection(0, 1, 0), 0);
    }

    #[test]
    fn clean_tab_steers_between_docked_and_undocked_by_swap_name() {
        // Installed order is [BASE, docked, undocked]; steering by the
        // reported name avoids the position quirks of blind cycling around
        // BASE and the end of the list.
        let instances = [inst(7, 1, false)];
        let steer = |name: Option<&str>| decide_toggle(Some(1), false, Some(1), name, false, false, 7, &instances);
        assert_eq!(
            steer(Some("docked")),
            ToggleAction::SteerSwap { backwards: false }
        );
        assert_eq!(
            steer(Some("undocked")),
            ToggleAction::SteerSwap { backwards: true }
        );
        // BASE is geometrically identical to docked on template-born tabs, so
        // forwards (BASE -> docked) is a dead press; backwards from position 0
        // wraps to the list end (undocked) — a visible collapse.
        assert_eq!(
            steer(Some("BASE")),
            ToggleAction::SteerSwap { backwards: true }
        );
        assert_eq!(steer(None), ToggleAction::SteerSwap { backwards: true });
    }

    #[test]
    fn runtime_toggle_route_targets_the_resident_plugin_without_launching() {
        assert_eq!(
            runtime_toggle_keybind_kdl(42),
            "keybinds {\n    shared {\n        bind \"Alt /\" {\n            MessagePluginId 42 {\n                name \"toggle\"\n            }\n        }\n    }\n}\n"
        );
    }

    #[test]
    fn observed_active_tiled_keybind_pipe_is_authorized_without_a_route_flag() {
        // `reconfigure()` has no acknowledgement. The actual keybind pipe is
        // the authorization evidence: once it arrives, the active tiled rail
        // may toggle even if no local "route installed" flag was set.
        assert!(should_accept_observed_toggle_pipe(
            true,
            &PipeSource::Keybind,
            false,
            Some(3),
            Some(3)
        ));
        assert!(!should_accept_observed_toggle_pipe(
            true,
            &PipeSource::Cli("not-a-keybind".to_owned()),
            false,
            Some(3),
            Some(3)
        ));
        assert!(!should_accept_observed_toggle_pipe(
            true,
            &PipeSource::Keybind,
            true,
            Some(3),
            Some(3)
        ));
        assert!(!should_accept_observed_toggle_pipe(
            true,
            &PipeSource::Keybind,
            false,
            Some(3),
            Some(4)
        ));
    }

    #[test]
    fn only_an_active_tiled_rail_can_install_the_runtime_toggle_route() {
        assert!(should_route_toggle_to_self(
            false,
            true,
            false,
            Some(3),
            Some(3)
        ));
        assert!(!should_route_toggle_to_self(
            true,
            true,
            false,
            Some(3),
            Some(3)
        ));
        assert!(!should_route_toggle_to_self(
            false,
            false,
            false,
            Some(3),
            Some(3)
        ));
        assert!(!should_route_toggle_to_self(
            false,
            true,
            true,
            Some(3),
            Some(3)
        ));
        assert!(!should_route_toggle_to_self(
            false,
            true,
            false,
            Some(3),
            Some(4)
        ));
        assert!(!should_route_toggle_to_self(
            false,
            true,
            false,
            None,
            None
        ));
    }

    #[test]
    fn foreign_swap_set_is_replaced_not_cycled() {
        // A clean tab can carry a swap set that is not ours — a builtin or
        // user-captured one ("vertical", "stacked"). Steering would cycle
        // those foreign templates, re-tiling the user's panes while the dock
        // never toggles, and pending_steer_disposition drops a foreign name
        // anyway. Rebuild the swap set around the current arrangement
        // instead, exactly like a dirty tab.
        let instances = [inst(7, 1, false)];
        let toggle = |name: Option<&str>| {
            decide_toggle(Some(1), false, Some(1), name, false, false, 7, &instances)
        };
        assert_eq!(
            toggle(Some("vertical")),
            ToggleAction::RegenerateSwaps {
                target: DockState::Undocked
            }
        );
        assert_eq!(
            toggle(Some("stacked")),
            ToggleAction::RegenerateSwaps {
                target: DockState::Undocked
            }
        );
    }

    #[test]
    fn toggle_is_ignored_while_floating_panes_are_visible() {
        // While a tab's floating panes are visible, swap_name and swap_dirty
        // speak for the FLOATING layer (TabState), whose birth entry is the
        // allowlisted "BASE" — an immediate swap step would cycle the
        // floating layer while the tiled dock state stays unreadable. The
        // press is dropped, not parked: pending_steer_disposition likewise
        // refuses to read the tiled outcome while floats are visible.
        let instances = [inst(7, 1, false)];
        let toggle = |name: Option<&str>, dirty: bool| {
            decide_toggle(Some(1), false, Some(1), name, dirty, true, 7, &instances)
        };
        assert_eq!(toggle(Some("BASE"), false), ToggleAction::Ignore);
        assert_eq!(toggle(Some("docked"), false), ToggleAction::Ignore);
        assert_eq!(toggle(Some("docked"), true), ToggleAction::Ignore);
    }

    #[test]
    fn dirty_tab_regenerates_swaps_toward_the_other_state() {
        // A damaged tab (manual split/resize) would snap-fold to a stale
        // template on the next swap; instead the swap set is rebuilt around
        // the current arrangement and steered to the opposite state.
        let instances = [inst(7, 1, false)];
        let regen = |name: Option<&str>| decide_toggle(Some(1), false, Some(1), name, true, false, 7, &instances);
        assert_eq!(
            regen(Some("docked")),
            ToggleAction::RegenerateSwaps {
                target: DockState::Undocked
            }
        );
        assert_eq!(
            regen(Some("undocked")),
            ToggleAction::RegenerateSwaps {
                target: DockState::Docked
            }
        );
        assert_eq!(
            regen(Some("BASE")),
            ToggleAction::RegenerateSwaps {
                target: DockState::Undocked
            }
        );
    }

    #[test]
    fn defers_to_the_resident_instance_of_the_active_tab() {
        let instances = [inst(7, 1, false), inst(9, 3, false)];
        assert_eq!(
            decide_toggle(Some(1), false, Some(3), None, false, false, 7, &instances),
            ToggleAction::Ignore
        );
    }

    #[test]
    fn sidebarless_foreign_active_tab_is_inert() {
        // Alt / is not an entry or retrofit operation. A resident that sees a
        // foreign, sidebar-less active tab must leave it completely alone.
        let instances = [inst(7, 1, false), inst(9, 2, false)];
        assert_eq!(
            decide_toggle(Some(1), false, Some(5), None, false, false, 7, &instances),
            ToggleAction::Ignore
        );
        assert_eq!(
            decide_toggle(Some(2), false, Some(5), None, false, false, 9, &instances),
            ToggleAction::Ignore
        );
    }

    #[test]
    fn stale_active_tab_view_never_initializes_a_foreign_tab() {
        let instances = [inst(14, 2, false), inst(16, 5, false)];
        assert_eq!(
            decide_toggle(Some(5), false, Some(6), None, false, false, 16, &instances),
            ToggleAction::Ignore
        );
    }

    #[test]
    fn ignores_toggle_when_active_tab_is_unknown() {
        assert_eq!(
            decide_toggle(
                Some(1),
                false,
                None,
                None,
                false,
                false,
                7,
                &[inst(7, 1, false)]
            ),
            ToggleAction::Ignore
        );
    }

    #[test]
    fn floating_sidebar_cannot_initialize_an_active_tab() {
        let instances = [inst(7, 1, true)];
        assert_eq!(
            decide_toggle(Some(1), true, Some(1), None, false, false, 7, &instances),
            ToggleAction::Ignore
        );
    }

    #[test]
    fn floating_resident_defers_to_the_tabs_tiled_sidebar() {
        // If a legacy layout leaves both a floating and tiled rail in a tab,
        // only the tiled one may act.
        let instances = [inst(7, 1, true), inst(9, 1, false)];
        assert_eq!(
            decide_toggle(Some(1), true, Some(1), None, false, false, 7, &instances),
            ToggleAction::Ignore
        );
        assert_eq!(
            decide_toggle(Some(1), false, Some(1), Some("docked"), false, false, 9, &instances),
            ToggleAction::SteerSwap { backwards: false }
        );
    }

    #[test]
    fn floating_sidebars_never_become_toggle_actors() {
        let instances = [inst(7, 1, true), inst(9, 1, true)];
        assert_eq!(
            decide_toggle(Some(1), true, Some(1), None, false, false, 7, &instances),
            ToggleAction::Ignore
        );
        assert_eq!(
            decide_toggle(Some(1), true, Some(1), None, false, false, 9, &instances),
            ToggleAction::Ignore
        );
    }

    #[test]
    fn floating_bystander_defers_to_any_active_tab_resident() {
        // A floating bystander in tab 2 never acts on another tab.
        let instances = [inst(7, 2, true), inst(9, 1, false)];
        assert_eq!(
            decide_toggle(Some(2), true, Some(1), None, false, false, 7, &instances),
            ToggleAction::Ignore
        );
        let instances = [inst(7, 2, true), inst(9, 1, true)];
        assert_eq!(
            decide_toggle(Some(2), true, Some(1), None, false, false, 7, &instances),
            ToggleAction::Ignore
        );
    }

    #[test]
    fn floating_bystander_leaves_a_sidebarless_tab_unchanged() {
        let instances = [inst(7, 2, true), inst(9, 3, false)];
        assert_eq!(
            decide_toggle(Some(2), true, Some(5), None, false, false, 7, &instances),
            ToggleAction::Ignore
        );
        assert_eq!(
            decide_toggle(Some(3), false, Some(5), None, false, false, 9, &instances),
            ToggleAction::Ignore
        );
    }

    #[test]
    fn remote_instance_never_toggles_even_when_the_swap_name_looks_ours() {
        // A manifest can name a swap state before a resident is observed, but
        // a remote sidebar is still not permission to touch that tab.
        let instances = [inst(7, 2, false)];
        assert_eq!(
            decide_toggle(Some(2), false, Some(5), Some("docked"), false, false, 7, &instances),
            ToggleAction::Ignore
        );
        assert_eq!(
            decide_toggle(Some(2), false, Some(5), Some("undocked"), false, false, 7, &instances),
            ToggleAction::Ignore
        );
        assert_eq!(
            decide_toggle(
                Some(2),
                false,
                Some(5),
                Some("BASE"),
                false,
                false,
                7,
                &instances
            ),
            ToggleAction::Ignore
        );
        assert_eq!(
            decide_toggle(Some(2), false, Some(5), None, false, false, 7, &instances),
            ToggleAction::Ignore
        );
    }

    #[test]
    fn tab_update_records_floating_pane_visibility() {
        // While a tab's floating panes are visible, TabInfo's swap name
        // reports the FLOATING layer (tab/mod.rs swap_layout_info), so the
        // tiled override outcome is unreadable; the state must carry the
        // visibility bit for the steer to know whose name it is reading.
        let mut sidebar = Sidebar::default();
        let mut with_floats = tab_info(1, 7, true, Some("BASE"), false);
        with_floats.are_floating_panes_visible = true;
        sidebar.update(Event::TabUpdate(vec![
            tab_info(0, 5, false, Some("BASE"), false),
            with_floats,
        ]));
        assert!(!sidebar.tab_states.get(&0).unwrap().floating_visible);
        assert!(sidebar.tab_states.get(&1).unwrap().floating_visible);
    }

    #[test]
    fn tab_update_records_the_reported_active_tab_for_the_poll_gate() {
        // The poll gate reads reported_active_tab, sourced only from
        // TabUpdate's server-authoritative t.active — not active_tab, which
        // the toggle path overwrites with the flaky get_focused_pane_info
        // translation.
        let mut sidebar = Sidebar::default();
        sidebar.update(Event::TabUpdate(vec![
            tab_info(0, 5, false, None, false),
            tab_info(2, 20, true, None, false),
        ]));
        assert_eq!(sidebar.reported_active_tab, Some(2));
        assert_eq!(sidebar.active_tab, Some(2));
    }

    #[test]
    fn tab_update_records_swap_state_per_tab() {
        let mut sidebar = Sidebar::default();
        sidebar.update(Event::TabUpdate(vec![
            tab_info(0, 5, false, Some("BASE"), false),
            tab_info(1, 7, true, Some("docked"), true),
        ]));
        assert_eq!(sidebar.active_tab, Some(1));
        assert_eq!(
            sidebar.tab_states.get(&0),
            Some(&TabState {
                id: 5,
                swap_name: Some("BASE".to_owned()),
                swap_dirty: false,
                floating_visible: false,
            })
        );
        assert_eq!(
            sidebar.tab_states.get(&1),
            Some(&TabState {
                id: 7,
                swap_name: Some("docked".to_owned()),
                swap_dirty: true,
                floating_visible: false,
            })
        );
    }

    fn steer_states(name: Option<&str>, floating_visible: bool) -> BTreeMap<usize, TabState> {
        let mut map = BTreeMap::new();
        map.insert(
            1,
            TabState {
                id: 4,
                swap_name: name.map(str::to_owned),
                // A landed override was observed live still flagged dirty;
                // the damage flag is no part of the signature.
                swap_dirty: true,
                floating_visible,
            },
        );
        map
    }

    fn steer_to(target: DockState) -> PendingSteer {
        PendingSteer { tab: 1, target }
    }

    #[test]
    fn pending_steer_completes_the_toggle_from_wherever_the_override_landed() {
        // A landed override does not leave the tab at BASE unconditionally:
        // zellij relayouts the tab right after installing the set, so the
        // report is BASE only when the absorb base's exact pane count
        // matches (single-shell tabs); multi-pane tabs arrive already at
        // "docked". The steer turns the reported entry into the one
        // deliberate step that reaches the target.
        assert_eq!(
            pending_steer_disposition(steer_to(DockState::Docked), Some(1), &steer_states(Some("BASE"), false)),
            SteerDisposition::Fire { backwards: false }
        );
        assert_eq!(
            pending_steer_disposition(steer_to(DockState::Undocked), Some(1), &steer_states(Some("BASE"), false)),
            SteerDisposition::Fire { backwards: true }
        );
        // The relayout stopped one entry short of the target: step onward.
        assert_eq!(
            pending_steer_disposition(steer_to(DockState::Undocked), Some(1), &steer_states(Some("docked"), false)),
            SteerDisposition::Fire { backwards: false }
        );
        assert_eq!(
            pending_steer_disposition(steer_to(DockState::Docked), Some(1), &steer_states(Some("undocked"), false)),
            SteerDisposition::Fire { backwards: true }
        );
        // The relayout already landed on the target: the toggle is done and
        // one more step would overshoot it (the live one-step-off arrival).
        assert_eq!(
            pending_steer_disposition(steer_to(DockState::Docked), Some(1), &steer_states(Some("docked"), false)),
            SteerDisposition::Drop
        );
        assert_eq!(
            pending_steer_disposition(steer_to(DockState::Undocked), Some(1), &steer_states(Some("undocked"), false)),
            SteerDisposition::Drop
        );
        // A foreign name means the override failed or raced; the press is
        // abandoned rather than kept armed forever.
        assert_eq!(
            pending_steer_disposition(steer_to(DockState::Docked), Some(1), &steer_states(Some("stacked"), false)),
            SteerDisposition::Drop
        );
        // No name at all is the one-selectable-pane blind spot; the
        // override's outcome is still unreadable, so the press stays armed.
        assert_eq!(
            pending_steer_disposition(steer_to(DockState::Docked), Some(1), &steer_states(None, false)),
            SteerDisposition::Keep
        );
        // The tab vanished (closed) — the press has nowhere to go.
        assert_eq!(
            pending_steer_disposition(
                PendingSteer { tab: 2, target: DockState::Docked },
                Some(1),
                &steer_states(Some("BASE"), false)
            ),
            SteerDisposition::Drop
        );
        // The user switched away: swap presses act on the client's active
        // tab, so firing now would steer the wrong tab. The regenerated
        // set stays installed; a later press on the tab steers by name.
        assert_eq!(
            pending_steer_disposition(steer_to(DockState::Docked), Some(3), &steer_states(Some("BASE"), false)),
            SteerDisposition::Drop
        );
    }

    #[test]
    fn steer_waits_while_floating_panes_obscure_the_swap_report() {
        // While floating panes are visible the reported name speaks for the
        // floating layer, whose birth entry is also "BASE" — during a
        // bootstrap retrofit the floater itself keeps the tab in that
        // state, and firing on it steers before the override lands.
        assert_eq!(
            pending_steer_disposition(steer_to(DockState::Docked), Some(1), &steer_states(Some("BASE"), true)),
            SteerDisposition::Keep
        );
        assert_eq!(
            pending_steer_disposition(steer_to(DockState::Docked), Some(1), &steer_states(Some("docked"), true)),
            SteerDisposition::Keep
        );
    }

    #[test]
    fn bootstrap_arrival_stays_docked_despite_floating_base_reports() {
        // The live E2 failure: retrofit override in flight, the floating
        // bootstrap pane still visible. Gap TabUpdates report the floating
        // layer's "BASE"; firing there adds a stray forward step on top of
        // the relayout's own docked arrival and the tab lands one entry
        // past it, on the undocked sliver.
        let mut sidebar = Sidebar::default();
        sidebar.pending_steer = Some(PendingSteer {
            tab: 1,
            target: DockState::Docked,
        });
        let mut gap = tab_info(1, 7, true, Some("BASE"), true);
        gap.are_floating_panes_visible = true;
        sidebar.update(Event::TabUpdate(vec![gap]));
        assert!(sidebar.pending_steer.is_some());
        assert!(sidebar.toggle_cooldown.is_none());
        // The override landed and hid the floater; the relayout already
        // reached docked, so the steer stands down without a stray step.
        sidebar.update(Event::TabUpdate(vec![tab_info(1, 7, true, Some("docked"), false)]));
        assert!(sidebar.pending_steer.is_none());
        assert!(sidebar.toggle_cooldown.is_none());
    }

    #[test]
    fn rebuild_targets_a_tab_by_server_id_only_when_dumpable() {
        // The dump host call is only safe post-grant, and it speaks the
        // server's stable tab id — not the display position the rest of
        // the plugin uses. A position TabUpdate has not reported has no
        // id to dump.
        let mut tab_states = BTreeMap::new();
        tab_states.insert(
            5,
            TabState {
                id: 42,
                ..Default::default()
            },
        );
        assert_eq!(rebuild_target(true, Some(5), &tab_states), Some((5, 42)));
        assert_eq!(rebuild_target(false, Some(5), &tab_states), None);
        assert_eq!(rebuild_target(true, Some(6), &tab_states), None);
        assert_eq!(rebuild_target(true, None, &tab_states), None);
    }

    #[test]
    fn tab_update_fires_the_pending_steer_once_the_override_lands() {
        let mut sidebar = Sidebar::default();
        sidebar.pending_steer = Some(PendingSteer {
            tab: 1,
            target: DockState::Undocked,
        });
        // The tab reports no swap name (one-selectable-pane blind spot):
        // the override's outcome is unreadable, the press stays armed.
        sidebar.update(Event::TabUpdate(vec![tab_info(1, 7, true, None, false)]));
        assert!(sidebar.pending_steer.is_some());
        // Override landed (BASE — still flagged dirty, as observed live):
        // the steer fires and clears.
        sidebar.update(Event::TabUpdate(vec![tab_info(
            1,
            7,
            true,
            Some("BASE"),
            true,
        )]));
        assert!(sidebar.pending_steer.is_none());
        // A press for a tab that no longer exists is abandoned.
        sidebar.pending_steer = Some(PendingSteer {
            tab: 9,
            target: DockState::Docked,
        });
        sidebar.update(Event::TabUpdate(vec![tab_info(
            1,
            7,
            true,
            Some("BASE"),
            false,
        )]));
        assert!(sidebar.pending_steer.is_none());
    }

    #[test]
    fn press_for_the_in_flight_tab_is_swallowed_others_supersede() {
        let now = Instant::now();
        let in_flight = Some(PendingSteer {
            tab: 1,
            target: DockState::Docked,
        });
        // Tab 1's pipeline is still waiting on its override: a repeat press
        // there is the bounce, not a new intent.
        assert!(should_swallow_toggle(Some(1), in_flight, None, now));
        // A press for another tab is a fresh intent and must act.
        assert!(!should_swallow_toggle(Some(2), in_flight, None, now));
        // No known active tab: nothing to debounce against.
        assert!(!should_swallow_toggle(None, in_flight, None, now));
        // Nothing armed at all.
        assert!(!should_swallow_toggle(Some(1), None, None, now));
    }

    #[test]
    fn press_within_the_cooldown_window_is_swallowed() {
        let fired = Instant::now();
        let cooldown = Some((1, fired));
        assert!(should_swallow_toggle(Some(1), None, cooldown, fired));
        assert!(should_swallow_toggle(
            Some(1),
            None,
            cooldown,
            fired + TOGGLE_COOLDOWN / 2
        ));
        // The window closes exactly at the cooldown bound.
        assert!(!should_swallow_toggle(
            Some(1),
            None,
            cooldown,
            fired + TOGGLE_COOLDOWN
        ));
        // Another tab is never held back by tab 1's cooldown.
        assert!(!should_swallow_toggle(
            Some(2),
            None,
            cooldown,
            fired + TOGGLE_COOLDOWN / 2
        ));
    }

    #[test]
    fn steer_fire_arms_the_cooldown_for_its_tab() {
        let mut sidebar = Sidebar::default();
        sidebar.pending_steer = Some(PendingSteer {
            tab: 1,
            target: DockState::Undocked,
        });
        sidebar.update(Event::TabUpdate(vec![tab_info(1, 7, true, Some("BASE"), false)]));
        assert!(sidebar.pending_steer.is_none());
        assert_eq!(sidebar.toggle_cooldown.map(|(tab, _)| tab), Some(1));

        // A dropped press arms nothing: no collapse is about to happen.
        let mut sidebar = Sidebar::default();
        sidebar.pending_steer = Some(PendingSteer {
            tab: 9,
            target: DockState::Docked,
        });
        sidebar.update(Event::TabUpdate(vec![tab_info(1, 7, true, Some("BASE"), false)]));
        assert!(sidebar.pending_steer.is_none());
        assert!(sidebar.toggle_cooldown.is_none());
    }

    #[test]
    fn floating_instance_toggle_does_not_rebuild_or_arm_a_cooldown() {
        // Only a tiled resident may act. A floating instance records neither
        // a rebuild nor a cooldown, even if it has a complete manifest.
        let mut sidebar = Sidebar::default();
        sidebar.plugin_id = 7;
        sidebar.active_tab = Some(1);
        sidebar.own_tab = Some(1);
        sidebar.own_floating = true;
        sidebar.own_url = Some("file:/tmp/zellij-sidebar.wasm".to_owned());
        sidebar.instances = vec![inst(7, 1, true)];
        sidebar.perform_toggle();
        assert!(sidebar.toggle_cooldown.is_none(), "deferring arms no cooldown");
        assert!(sidebar.pending_steer.is_none(), "deferring records no steer");
    }

    #[test]
    fn repeat_press_is_swallowed_only_for_its_in_flight_tab() {
        let steer = PendingSteer {
            tab: 1,
            target: DockState::Undocked,
        };
        let now = Instant::now();
        assert!(should_swallow_toggle(Some(1), Some(steer), None, now));
        // A press for another tab is not the parked steer's bounce and the
        // caller will supersede it before choosing its next action.
        assert!(!should_swallow_toggle(Some(2), Some(steer), None, now));
    }

    #[test]
    fn floating_instance_closes_once_its_tab_holds_a_tiled_sidebar() {
        // A floating actor beside a tiled rail lingers as an invisible
        // config-matched zombie unless it yields to the tiled sidebar.
        assert!(is_stray_floating_bootstrap(
            true,
            Some(1),
            &[inst(7, 1, true), inst(9, 1, false)]
        ));
        // The tiled sidebar lives in another tab: not superseded.
        assert!(!is_stray_floating_bootstrap(
            true,
            Some(1),
            &[inst(7, 1, true), inst(9, 2, false)]
        ));
        // A tiled instance never closes itself.
        assert!(!is_stray_floating_bootstrap(
            false,
            Some(1),
            &[inst(9, 1, false)]
        ));
        // Only floating siblings around: no tiled rail has landed.
        assert!(!is_stray_floating_bootstrap(
            true,
            Some(1),
            &[inst(7, 1, true), inst(9, 1, true)]
        ));
        // Transient manifest without an own tab yet: never fire.
        assert!(!is_stray_floating_bootstrap(true, None, &[inst(9, 1, false)]));
    }

    #[test]
    fn close_self_fires_only_once_per_instance_lifetime() {
        // close_self does not remove the pane from the very next manifest
        // snapshot the plugin sees (observed live: one instance logged four
        // "Bye" lines), so is_stray_floating_bootstrap can still read true
        // afterward; a fired flag must suppress every call after the first.
        let instances = [inst(7, 1, true), inst(9, 1, false)];
        assert!(should_close_self(false, 7, true, Some(1), &instances));
        assert!(!should_close_self(true, 7, true, Some(1), &instances));
        // A one-shot never re-arms, even once the manifest catches up and
        // the underlying condition reads false again.
        assert!(!should_close_self(true, 9, false, Some(1), &[inst(9, 1, false)]));
    }

    #[test]
    fn redundant_tiled_sidebar_is_the_higher_id_of_two_in_a_tab() {
        // The relaxed election can let two instances both dock a rail into
        // the same fresh tab in one flood window (the dump abort dedupes only
        // once a rail lands). If a tab ends up with two tiled rails, the
        // higher-id one is redundant and closes itself, leaving the lowest-id
        // resident. Both read the same manifest, so the choice is
        // deterministic.
        let two = [inst(7, 1, false), inst(9, 1, false)];
        assert!(is_redundant_tiled_sidebar(9, Some(1), false, &two), "higher id closes");
        assert!(!is_redundant_tiled_sidebar(7, Some(1), false, &two), "lowest id stays");
        // A lone rail is never redundant (the common single-rail case — the
        // regression guard).
        assert!(!is_redundant_tiled_sidebar(7, Some(1), false, &[inst(7, 1, false)]));
        // A floating sibling does not make a tiled rail redundant — that is
        // the stray-floater path, and the floater is the one that closes.
        let with_floater = [inst(7, 1, true), inst(9, 1, false)];
        assert!(!is_redundant_tiled_sidebar(9, Some(1), false, &with_floater));
        // A floating self uses the stray-floater path, not this one.
        assert!(!is_redundant_tiled_sidebar(9, Some(1), true, &[inst(7, 1, false), inst(9, 1, true)]));
    }

    #[test]
    fn close_self_also_fires_for_a_redundant_tiled_rail() {
        let two = [inst(7, 1, false), inst(9, 1, false)];
        assert!(should_close_self(false, 9, false, Some(1), &two), "higher-id rail closes");
        assert!(!should_close_self(false, 7, false, Some(1), &two), "lowest-id rail stays");
        // Still one-shot.
        assert!(!should_close_self(true, 9, false, Some(1), &two));
    }

    #[test]
    fn stray_floater_sets_the_close_requested_flag_once() {
        let mut sidebar = Sidebar::default();
        sidebar.plugin_id = 7;
        let m = manifest(vec![(
            1,
            vec![sidebar_pane(7, true), sidebar_pane(9, false)],
        )]);
        assert!(!sidebar.close_requested);
        sidebar.update(Event::PaneUpdate(m.clone()));
        assert!(sidebar.close_requested);
        // A second PaneUpdate with the same stray shape (the pane has not
        // been dropped from the manifest yet) must not panic or re-fire —
        // the flag alone gates it.
        sidebar.update(Event::PaneUpdate(m));
        assert!(sidebar.close_requested);
    }

    #[test]
    fn pane_update_tracks_own_floating_state() {
        let mut sidebar = Sidebar::default();
        sidebar.plugin_id = 7;
        let m = manifest(vec![(
            1,
            vec![sidebar_pane(7, true), pane(3, false, "shell", 2, false)],
        )]);
        sidebar.update(Event::PaneUpdate(m));
        assert!(sidebar.own_floating);

        let m = manifest(vec![(
            1,
            vec![sidebar_pane(7, false), pane(3, false, "shell", 2, false)],
        )]);
        sidebar.update(Event::PaneUpdate(m));
        assert!(!sidebar.own_floating);
    }

    #[test]
    fn instance_list_records_tab_and_floating_state() {
        let mut other_plugin = pane(4, true, "tab-bar", 0, false);
        other_plugin.plugin_url = Some("zellij:tab-bar".to_owned());
        let m = manifest(vec![
            (0, vec![sidebar_pane(5, false), other_plugin]),
            (
                2,
                vec![sidebar_pane(9, true), pane(3, false, "shell", 2, false)],
            ),
        ]);
        assert_eq!(
            sidebar_instances(&m),
            vec![inst(5, 0, false), inst(9, 2, true)]
        );
    }

    #[test]
    fn live_focused_tab_overrides_stale_cached_active_tab() {
        // get_focused_pane_info reports the server's stable tab id, which
        // only matches the display position until a tab is closed or moved;
        // the TabUpdate-derived states translate id -> position.
        let mut tab_states = BTreeMap::new();
        tab_states.insert(0, TabState::default());
        tab_states.insert(
            2,
            TabState {
                id: 3,
                ..Default::default()
            },
        );
        assert_eq!(
            active_tab_for_decision(Some(1), Ok((3, PaneId::Terminal(9))), &tab_states),
            Some(2)
        );
        assert_eq!(
            active_tab_for_decision(Some(1), Err("unavailable".to_owned()), &tab_states),
            Some(1)
        );
        // An id the TabUpdate has not caught up with falls back to the cache.
        assert_eq!(
            active_tab_for_decision(Some(1), Ok((9, PaneId::Terminal(9))), &tab_states),
            Some(1)
        );
    }

    #[test]
    fn header_clicks_anywhere_toggle_the_swap_layout() {
        let rows = vec![
            Row {
                pane_id: 4,
                ..Default::default()
            },
            Row {
                pane_id: 8,
                ..Default::default()
            },
        ];
        // The whole header line is one control: clicking it collapses or
        // expands the sidebar via the tab's swap layout, same as Alt-/.
        assert_eq!(decide_click(1, &rows), ClickAction::ToggleDock);
        assert_eq!(decide_click(2, &rows), ClickAction::FocusPane(4));
        assert_eq!(decide_click(5, &rows), ClickAction::FocusPane(8));
        assert_eq!(decide_click(99, &rows), ClickAction::None);
    }

    #[test]
    fn session_row_click_through_the_rail_focuses_and_exits_nav() {
        // The click handler routes through the sectioned decider: a bound
        // session row's line reaches FocusPane (observable here through the
        // nav exit it shares with pane-row clicks).
        let mut sidebar = Sidebar::default();
        sidebar.nav_mode = true;
        sidebar.rows = vec![cwd_row(4), cwd_row(8)];
        sidebar.pane_cwds = cwd_map(&[(4, "/w")]);
        sidebar.sessions = vec![SessionEvent {
            pane_id: Some(4),
            cwd: "/w".to_owned(),
            ..Default::default()
        }];
        sidebar.handle_click(7); // P=2,S=1: the session row's first line
        assert!(!sidebar.nav_mode, "the session click must reach FocusPane");
    }

    #[test]
    fn row_click_during_nav_mode_exits_nav() {
        // A row click while nav mode is on focuses the pane but must also
        // exit nav — otherwise nav_mode stays latched: the rail stays
        // selectable, the focus-handback guard stays disabled, and
        // Enter/Esc keep routing to the now-focused terminal (F8).
        let mut sidebar = Sidebar::default();
        sidebar.nav_mode = true;
        sidebar.rows = vec![Row {
            pane_id: 4,
            ..Default::default()
        }];
        sidebar.handle_click(2); // line 2 = row 0 → FocusPane(4)
        assert!(!sidebar.nav_mode, "click-through must drop nav mode");
    }

    #[test]
    fn pre_grant_active_tab_comes_from_the_cache_only() {
        // Before the ReadApplicationState grant the host writes no response
        // bytes for get_focused_pane_info and the blocking shim call panics;
        // the cached TabUpdate value is the only safe source.
        let mut sidebar = Sidebar::default();
        sidebar.active_tab = Some(2);
        assert_eq!(sidebar.current_active_tab(), Some(2));
    }

    #[test]
    fn hands_back_focus_only_when_another_pane_can_take_it() {
        assert!(should_hand_back_focus(true, false, true));
        // Session-birth window: no other selectable pane yet — bouncing focus
        // here panics the whole server (get_active_pane_id unwraps None).
        assert!(!should_hand_back_focus(true, false, false));
        assert!(!should_hand_back_focus(true, true, true)); // nav mode keeps focus
        assert!(!should_hand_back_focus(false, false, true));
    }

    #[test]
    fn navigate_enters_nav_only_in_the_resident_instance() {
        let navigate = || PipeMessage {
            source: PipeSource::Keybind,
            name: "navigate".to_owned(),
            payload: None,
            args: BTreeMap::new(),
            is_private: false,
        };
        // The pipe reaches every instance; only the active tab's resident
        // takes nav focus — otherwise every tab's sidebar would call
        // focus_plugin_pane and fight over the client.
        let mut resident = Sidebar::default();
        resident.own_tab = Some(1);
        resident.active_tab = Some(1);
        assert!(resident.pipe(navigate()));
        assert!(resident.nav_mode);

        let mut bystander = Sidebar::default();
        bystander.own_tab = Some(2);
        bystander.active_tab = Some(1);
        assert!(!bystander.pipe(navigate()));
        assert!(!bystander.nav_mode);
    }

    fn toggle() -> PipeMessage {
        PipeMessage {
            source: PipeSource::Keybind,
            name: "toggle".to_owned(),
            payload: None,
            args: BTreeMap::new(),
            is_private: false,
        }
    }

    fn sidebar_with_pending_foreign_steer() -> (Sidebar, PendingSteer) {
        let pending = PendingSteer {
            tab: 1,
            target: DockState::Undocked,
        };
        let sidebar = Sidebar {
            active_tab: Some(2),
            own_tab: Some(2),
            pending_steer: Some(pending),
            ..Default::default()
        };
        (sidebar, pending)
    }

    #[test]
    fn pregrant_toggle_pipe_is_inert_even_with_a_requested_route() {
        // A runtime route leaked from another client must not let an
        // unapproved client consume or mutate toggle work.
        let (mut sidebar, pending) = sidebar_with_pending_foreign_steer();
        sidebar.permissions_granted = false;
        sidebar.toggle_route_requested = true;

        sidebar.pipe(toggle());

        assert_eq!(sidebar.pending_steer, Some(pending));
    }

    #[test]
    fn unrouted_header_click_is_inert_even_after_permission() {
        // The visible header follows the same fail-closed route ownership as
        // Alt /; permission alone cannot make it a layout mutator.
        let (mut sidebar, pending) = sidebar_with_pending_foreign_steer();
        sidebar.permissions_granted = true;
        sidebar.toggle_route_requested = false;

        sidebar.handle_click(1); // header line

        assert_eq!(sidebar.pending_steer, Some(pending));
    }

    #[test]
    fn toggle_pipe_before_the_first_manifest_is_inert() {
        // A persisted Alt / keybind is fail-closed until a resident rail has
        // installed its current-client MessagePluginId route. A pipe that
        // reaches an unnamed instance must not become deferred entry work.
        let mut sidebar = Sidebar::default();
        sidebar.active_tab = Some(0);
        sidebar.pipe(toggle());
        assert!(sidebar.pending_steer.is_none());
        assert!(sidebar.toggle_cooldown.is_none());
    }

    #[test]
    fn session_birth_manifest_with_only_the_sidebar_never_bounces_focus() {
        // A layout that focuses the sidebar produces a birth manifest whose
        // tab holds no other selectable pane; handing focus back then makes
        // the server unwrap a missing active pane and the session dies. The
        // handler derives has_focus_target from the row list.
        let m = manifest(vec![(0, vec![pane(7, true, "sidebar", 0, true)])]);
        let rows = rows_for_own_tab(&m, 7);
        assert!(!should_hand_back_focus(true, false, !rows.is_empty()));
    }

    // A dump_session_layout_for_tab result as zellij v0.44 serializes it:
    // the requesting plugin's own rail already removed by the server, chrome
    // present, nested percent containers, cwds, focus on the focused pane,
    // a floating_panes section, and the session's default-layout template
    // and swap sections trailing the tab node.
    fn dump_fixture() -> &'static str {
        r#"layout {
    tab name="Tab #1" focus=true hide_floating_panes=true {
        pane size=1 borderless=true {
            plugin location="zellij:tab-bar"
        }
        pane size="96%" split_direction="vertical" {
            pane name="build" cwd="/Users/clkao/git/zaphod" focus=true size="62%"
            pane size="38%" {
                pane cwd="/tmp" size="50%"
                pane command="claude" cwd="/Users/clkao" size="50%" {
                    args "--continue"
                }
            }
        }
        pane size=1 borderless=true {
            plugin location="zellij:status-bar"
        }
        floating_panes {
            pane {
                x 10
                y 5
                width 86
                height 22
            }
        }
    }
    new_tab_template {
        pane size=1 borderless=true {
            plugin location="zellij:tab-bar"
        }
        pane
        pane size=1 borderless=true {
            plugin location="zellij:status-bar"
        }
    }
    swap_tiled_layout name="stacked" {
        tab min_panes=5 {
            pane stacked=true {
                children
            }
        }
    }
}
"#
    }

    fn jit_config() -> BTreeMap<String, String> {
        let mut config = BTreeMap::new();
        config.insert("rail".to_owned(), "1".to_owned());
        config
    }

    #[test]
    fn split_preserving_layout_wraps_the_dumped_arrangement_in_both_swaps() {
        let kdl = split_preserving_layout_kdl(
            dump_fixture(),
            "file:/tmp/zellij-sidebar.wasm",
            &jit_config(),
        )
        .unwrap();
        assert!(kdl.contains("swap_tiled_layout name=\"docked\""));
        assert!(kdl.contains("swap_tiled_layout name=\"undocked\""));
        // Rail slot: 28 cols in the docked swap and the base tab, 1 col in
        // the undocked swap; the plugin block carries the config identity.
        assert_eq!(
            kdl.matches("pane size=28 borderless=true name=\"sidebar\"")
                .count(),
            2
        );
        assert_eq!(
            kdl.matches("pane size=1 borderless=true name=\"sidebar\"")
                .count(),
            1
        );
        assert_eq!(
            kdl.matches("plugin location=\"file:/tmp/zellij-sidebar.wasm\"")
                .count(),
            3
        );
        assert_eq!(kdl.matches("rail \"1\"").count(), 3);
        // Each chrome pane the dump shows becomes one canonical row in all
        // three tab bodies, so base and swaps stay chrome-consistent with
        // the live tab.
        assert_eq!(kdl.matches("zellij:tab-bar").count(), 3);
        assert_eq!(kdl.matches("zellij:status-bar").count(), 3);
        // The user's arrangement appears in both swaps with splits, sizes,
        // cwds and running commands intact.
        assert_eq!(kdl.matches("size=\"62%\"").count(), 2);
        assert_eq!(kdl.matches("size=\"38%\"").count(), 2);
        assert_eq!(kdl.matches("cwd=\"/tmp\"").count(), 2);
        assert_eq!(kdl.matches("command=\"claude\"").count(), 2);
        assert_eq!(kdl.matches("args \"--continue\"").count(), 2);
        // The base absorbs every pane into a stack; swaps re-seat from it.
        assert!(kdl.contains("pane stacked=true"));
        assert!(kdl.contains("children"));
        // The rail is the left column of the vertical split.
        assert!(kdl.find("name=\"sidebar\"").unwrap() < kdl.find("size=\"62%\"").unwrap());
    }

    #[test]
    fn split_preserving_layout_strips_volatile_dump_attributes() {
        let kdl = split_preserving_layout_kdl(
            dump_fixture(),
            "file:/tmp/zellij-sidebar.wasm",
            &jit_config(),
        )
        .unwrap();
        // focus in an override races the client's real focus; names would
        // pin user titles; the tab name would overwrite the user's on apply
        // (generated tab nodes stay unnamed so the live name survives).
        assert!(!kdl.contains("focus=true"));
        assert!(!kdl.contains("name=\"build\""));
        assert!(!kdl.contains("tab name="));
        assert!(!kdl.contains("hide_floating_panes"));
        // The region container fills whatever remains next to the rail; its
        // dumped size spoke for the pre-rail-removal geometry.
        assert!(!kdl.contains("size=\"96%\""));
        // Floating panes are never part of the tiled swap story.
        assert!(!kdl.contains("floating_panes"));
        assert!(!kdl.contains("width 86"));
        // The session's own template and swap sections do not ride along.
        assert!(!kdl.contains("new_tab_template"));
        assert!(!kdl.contains("min_panes=5"));
        assert!(!kdl.contains("swap_tiled_layout name=\"stacked\""));
    }

    #[test]
    fn split_preserving_layout_rejects_dumps_it_cannot_rebuild_from() {
        let config = jit_config();
        // No tab node at all (e.g. a stale tab id matched nothing).
        assert!(split_preserving_layout_kdl("layout {\n}\n", "file:/x.wasm", &config).is_err());
        // Chrome-only tab: nothing of the user's to preserve.
        let chrome_only = r#"layout {
    tab name="Tab #2" {
        pane size=1 borderless=true {
            plugin location="zellij:tab-bar"
        }
        pane size=1 borderless=true {
            plugin location="zellij:status-bar"
        }
    }
}
"#;
        assert!(split_preserving_layout_kdl(chrome_only, "file:/x.wasm", &config).is_err());
    }

    #[test]
    fn sibling_region_rows_stay_ordered_inside_one_flexible_container() {
        // A tab whose root splits top-to-bottom dumps the rows as siblings
        // of the chrome; they must land next to the rail as one container
        // keeping their order and relative sizes.
        let dump = r#"layout {
    tab name="Tab #3" {
        pane size=1 borderless=true {
            plugin location="zellij:tab-bar"
        }
        pane cwd="/upper" size="55%"
        pane cwd="/lower" size="43%"
        pane size=1 borderless=true {
            plugin location="zellij:status-bar"
        }
    }
}
"#;
        let kdl = split_preserving_layout_kdl(dump, "file:/x.wasm", &jit_config()).unwrap();
        assert_eq!(kdl.matches("cwd=\"/upper\" size=\"55%\"").count(), 2);
        assert_eq!(kdl.matches("cwd=\"/lower\" size=\"43%\"").count(), 2);
        assert!(kdl.find("/upper").unwrap() < kdl.find("/lower").unwrap());
        // Wrapped, not spliced bare into the vertical split: the two rows
        // would otherwise become columns.
        let docked = &kdl[..kdl.find("undocked").unwrap()];
        let wrapper = docked.find("pane {").expect("row wrapper");
        assert!(wrapper > docked.find("split_direction=\"vertical\"").unwrap());
    }

    #[test]
    fn stacked_arrangements_keep_their_stack_flags() {
        let dump = r#"layout {
    tab name="Tab #4" {
        pane size=1 borderless=true {
            plugin location="zellij:tab-bar"
        }
        pane stacked=true {
            pane cwd="/a"
            pane cwd="/b" expanded=true
            pane cwd="/c"
        }
        pane size=1 borderless=true {
            plugin location="zellij:status-bar"
        }
    }
}
"#;
        let kdl = split_preserving_layout_kdl(dump, "file:/x.wasm", &jit_config()).unwrap();
        // Both swaps carry the stack (plus the base's absorb stack).
        assert_eq!(kdl.matches("pane stacked=true").count(), 3);
        assert_eq!(kdl.matches("expanded=true").count(), 2);
    }

    #[test]
    fn chrome_baked_into_a_quadrant_is_reemitted_as_a_canonical_row() {
        // Observed live: zellij's absorb seated the tab-bar in a quadrant
        // slot (pane size="50%" borderless=true { zellij:tab-bar }) with no
        // top chrome row. The transform must pull it out wherever it sits
        // and emit one canonical top row per tab body — baking it into the
        // swaps as a user pane mangles the tab permanently.
        let dump = r#"layout {
    tab name="Tab #1" focus=true hide_floating_panes=true {
        pane split_direction="vertical" size="98%" {
            pane size="50%" {
                pane cwd="/a" size="50%"
                pane cwd="/b" size="50%"
            }
            pane size="50%" {
                pane cwd="/c" size="34%"
                pane size="66%" borderless=true {
                    plugin location="zellij:tab-bar"
                }
            }
        }
        pane size=1 borderless=true {
            plugin location="zellij:status-bar"
        }
    }
}
"#;
        let kdl = split_preserving_layout_kdl(dump, "file:/x.wasm", &jit_config()).unwrap();
        let tab_bar_row = "pane size=1 borderless=true {\nplugin location=\"zellij:tab-bar\"\n}\n";
        let status_bar_row =
            "pane size=1 borderless=true {\nplugin location=\"zellij:status-bar\"\n}\n";
        assert_eq!(kdl.matches(tab_bar_row).count(), 3);
        assert_eq!(kdl.matches("zellij:tab-bar").count(), 3);
        assert_eq!(kdl.matches(status_bar_row).count(), 3);
        // The quadrant seat is gone entirely, not left as an empty slot.
        assert!(!kdl.contains("size=\"66%\""));
        // The chrome pane's lone sibling takes over the container's slot.
        assert_eq!(kdl.matches("cwd=\"/c\" size=\"50%\"").count(), 2);
        assert!(!kdl.contains("size=\"34%\""));
        // Chrome frames the user region: tab-bar above it, status-bar below.
        assert!(kdl.find("zellij:tab-bar").unwrap() < kdl.find("cwd=\"/a\"").unwrap());
        assert!(kdl.find("cwd=\"/a\"").unwrap() < kdl.find("zellij:status-bar").unwrap());
    }

    #[test]
    fn chrome_inside_a_stack_is_extracted_and_reemitted() {
        // Observed live on a hand-split-then-retrofit tab: the tab-bar
        // arrived as a stack pane. It must leave the stack and become the
        // canonical top row; the stack keeps its surviving panes.
        let dump = r#"layout {
    tab name="Tab #2" {
        pane stacked=true {
            pane cwd="/a"
            pane cwd="/b" expanded=true
            pane borderless=true {
                plugin location="zellij:tab-bar"
            }
        }
        pane size=1 borderless=true {
            plugin location="zellij:status-bar"
        }
    }
}
"#;
        let kdl = split_preserving_layout_kdl(dump, "file:/x.wasm", &jit_config()).unwrap();
        let tab_bar_row = "pane size=1 borderless=true {\nplugin location=\"zellij:tab-bar\"\n}\n";
        assert_eq!(kdl.matches(tab_bar_row).count(), 3);
        assert_eq!(kdl.matches("zellij:tab-bar").count(), 3);
        assert_eq!(kdl.matches("zellij:status-bar").count(), 3);
        // The stack survives with its two panes (plus the base absorb stack).
        assert_eq!(kdl.matches("pane stacked=true").count(), 3);
        assert_eq!(kdl.matches("cwd=\"/a\"").count(), 2);
        assert_eq!(kdl.matches("expanded=true").count(), 2);
        assert!(!kdl.contains("pane borderless=true"));
    }

    #[test]
    fn status_bar_absorbed_into_a_stack_is_reemitted_as_the_bottom_row() {
        // Observed live on a hand-split tab after an absorb: the status-bar
        // seated INSIDE the stack as its last pane, with no tab-level
        // bottom row and the tab-bar row intact. A rebuild dump arrives
        // with that damage and must leave it repaired: exactly one
        // canonical status-bar row per tab body, and the stack keeps only
        // the user panes.
        let dump = r#"layout {
    tab name="Tab #2" {
        pane size=1 borderless=true {
            plugin location="zellij:tab-bar"
        }
        pane stacked=true {
            pane cwd="/a"
            pane cwd="/b"
            pane cwd="/c"
            pane cwd="/d" expanded=true
            pane borderless=true {
                plugin location="zellij:status-bar"
            }
        }
    }
}
"#;
        let kdl = split_preserving_layout_kdl(dump, "file:/x.wasm", &jit_config()).unwrap();
        let status_bar_row =
            "pane size=1 borderless=true {\nplugin location=\"zellij:status-bar\"\n}\n";
        let tab_bar_row = "pane size=1 borderless=true {\nplugin location=\"zellij:tab-bar\"\n}\n";
        assert_eq!(kdl.matches(status_bar_row).count(), 3);
        assert_eq!(kdl.matches("zellij:status-bar").count(), 3);
        assert_eq!(kdl.matches(tab_bar_row).count(), 3);
        assert_eq!(kdl.matches("zellij:tab-bar").count(), 3);
        // The stack survives with its four user panes and nothing else
        // (plus the base's absorb stack); its status-bar seat is gone.
        assert_eq!(kdl.matches("pane stacked=true").count(), 3);
        assert_eq!(kdl.matches("cwd=\"/a\"").count(), 2);
        assert_eq!(kdl.matches("cwd=\"/d\" expanded=true").count(), 2);
        assert!(!kdl.contains("pane borderless=true {"));
        // Chrome frames the user region: tab-bar above, status-bar below.
        assert!(kdl.find("zellij:tab-bar").unwrap() < kdl.find("cwd=\"/a\"").unwrap());
        assert!(kdl.find("cwd=\"/a\"").unwrap() < kdl.find("zellij:status-bar").unwrap());
    }

    #[test]
    fn container_emptied_by_chrome_extraction_is_dropped() {
        // A container whose only content was chrome must vanish with it,
        // and the cascade may leave its parent with a single child, which
        // then takes over the parent's slot.
        let dump = r#"layout {
    tab name="Tab #3" {
        pane size=1 borderless=true {
            plugin location="zellij:tab-bar"
        }
        pane split_direction="vertical" {
            pane cwd="/only" size="70%"
            pane size="30%" {
                pane size=1 borderless=true {
                    plugin location="zellij:status-bar"
                }
            }
        }
    }
}
"#;
        let kdl = split_preserving_layout_kdl(dump, "file:/x.wasm", &jit_config()).unwrap();
        assert_eq!(kdl.matches("zellij:tab-bar").count(), 3);
        assert_eq!(kdl.matches("zellij:status-bar").count(), 3);
        assert_eq!(kdl.matches("cwd=\"/only\"").count(), 2);
        // The emptied 30% wrapper and its parent container are both gone:
        // the only remaining vertical splits are the generated rail splits.
        assert_eq!(kdl.matches("split_direction=\"vertical\"").count(), 3);
        assert!(!kdl.contains("size=\"30%\""));
        assert!(!kdl.contains("size=\"70%\""));
    }

    // The docked tab body of a generated layout, sliced from the marker
    // onward and parsed with extract_tab_body itself — the swap's inner
    // content is a `tab { ... }` node, the same shape extract_tab_body
    // expects from a real dump.
    fn docked_tab_body(kdl: &str) -> Vec<String> {
        let idx = kdl
            .find("swap_tiled_layout name=\"docked\"")
            .expect("docked swap present");
        extract_tab_body(&kdl[idx..]).expect("docked swap wraps a tab node")
    }

    fn normalize(lines: &[String]) -> Vec<String> {
        lines
            .iter()
            .map(|line| line.trim().to_owned())
            .filter(|line| !line.is_empty())
            .collect()
    }

    #[test]
    fn single_line_chrome_extracts_like_multi_line() {
        // zellij dumps a chrome pane on ONE line:
        //   pane size=1 borderless=true { plugin location="zellij:tab-bar" }
        // — the format the transform actually receives in production (the
        // other fixtures use the non-production multi-line shape, which hid
        // this bug). Single-line chrome must extract to the SAME clean
        // template as multi-line: chrome hoisted to canonical top/bottom
        // rows, never baked into the region.
        let region = concat!(
            "        pane split_direction=\"vertical\" {\n",
            "            pane size=\"50%\" {\n",
            "                pane size=\"50%\" cwd=\"/a\"\n",
            "                pane size=\"50%\" cwd=\"/b\"\n",
            "            }\n",
            "            pane size=\"50%\" split_direction=\"vertical\" {\n",
            "                pane size=\"33%\" cwd=\"/c\"\n",
            "                pane size=\"33%\" cwd=\"/d\"\n",
            "                pane size=\"34%\" cwd=\"/e\"\n",
            "            }\n",
            "        }",
        );
        let wrap = |tabbar: &str, statusbar: &str| {
            format!("layout {{\n    tab name=\"t\" {{\n{tabbar}\n{region}\n{statusbar}\n    }}\n}}\n")
        };
        let multi = wrap(
            "        pane size=1 borderless=true {\n            plugin location=\"zellij:tab-bar\"\n        }",
            "        pane size=1 borderless=true {\n            plugin location=\"zellij:status-bar\"\n        }",
        );
        let single = wrap(
            "        pane size=1 borderless=true { plugin location=\"zellij:tab-bar\" }",
            "        pane size=1 borderless=true { plugin location=\"zellij:status-bar\" }",
        );
        let url = "file:/tmp/zellij-sidebar.wasm";
        let out_multi = split_preserving_layout_kdl(&multi, url, &jit_config()).unwrap();
        let out_single = split_preserving_layout_kdl(&single, url, &jit_config()).unwrap();
        assert_eq!(
            out_single, out_multi,
            "single-line chrome must extract identically to multi-line"
        );
        // The tab-bar is the tab's first child (top chrome row), immediately
        // before the region's vertical split — never baked in the region.
        assert!(out_single.contains(
            "tab {\npane size=1 borderless=true {\nplugin location=\"zellij:tab-bar\"\n}\npane split_direction=\"vertical\""
        ));
    }

    // A dumped tab carrying a rail, as zellij serializes it live: the rail
    // and chrome panes inline on ONE line each, the tab-bar baked mid-region
    // in a mixed vertical/horizontal split.
    fn single_line_rail_dump() -> &'static str {
        r#"layout {
    tab name="Tab #7" focus=true hide_floating_panes=true {
        pane split_direction="vertical" {
            pane name="sidebar" size=28 borderless=true { plugin location="file:/tmp/zellij-sidebar.wasm" { rail "1" } }
            pane {
                pane size="50%" split_direction="vertical" {
                    pane size="25%" cwd="/a"
                    pane size="25%" borderless=true { plugin location="zellij:tab-bar" }
                    pane size="50%" cwd="/b"
                }
                pane size="50%" split_direction="vertical" {
                    pane size="50%" cwd="/c"
                    pane focus=true size="50%" cwd="/d"
                }
            }
        }
        pane size=1 borderless=true { plugin location="zellij:status-bar" }
    }
}
"#
    }

    #[test]
    fn broken_mixed_split_dump_recovers_chrome_and_drops_stray_rail() {
        // The live tab-7 dump after the bug: a single-line tab-bar baked deep
        // in a mixed v/h region and a single-line stray rail left in the
        // region. Re-transforming it must hoist the tab-bar to a top row and
        // drop the stray rail — exactly one canonical rail per swap body.
        let url = "file:/tmp/zellij-sidebar.wasm";
        let out = split_preserving_layout_kdl(single_line_rail_dump(), url, &jit_config()).unwrap();
        assert!(out.contains(
            "tab {\npane size=1 borderless=true {\nplugin location=\"zellij:tab-bar\"\n}\npane split_direction=\"vertical\""
        ));
        assert_eq!(out.matches(url).count(), 3, "one canonical rail per swap body, stray dropped");
        assert_eq!(out.matches("zellij:tab-bar").count(), 3);
        assert_eq!(out.matches("zellij:status-bar").count(), 3);
    }

    #[test]
    fn single_line_chrome_regenerates_to_a_fixed_point() {
        // zellij dumps chrome single-line, so a re-toggle feeds the transform
        // its own output re-serialized single-line. That must be a fixed
        // point: the single-line form of the already-retrofitted shape
        // transforms to the same body as its multi-line form.
        let multi = r#"layout {
    tab name="Tab #3" {
        pane size=1 borderless=true {
            plugin location="zellij:tab-bar"
        }
        pane split_direction="vertical" {
            pane size=28 borderless=true name="sidebar" {
                plugin location="file:/tmp/zellij-sidebar.wasm" {
                    rail "1"
                }
            }
            pane cwd="/a" size="50%"
            pane cwd="/b" size="50%"
        }
        pane size=1 borderless=true {
            plugin location="zellij:status-bar"
        }
    }
}
"#;
        let single = r#"layout {
    tab name="Tab #3" {
        pane size=1 borderless=true { plugin location="zellij:tab-bar" }
        pane split_direction="vertical" {
            pane size=28 borderless=true name="sidebar" { plugin location="file:/tmp/zellij-sidebar.wasm" { rail "1" } }
            pane cwd="/a" size="50%"
            pane cwd="/b" size="50%"
        }
        pane size=1 borderless=true { plugin location="zellij:status-bar" }
    }
}
"#;
        let url = "file:/tmp/zellij-sidebar.wasm";
        let out_multi = split_preserving_layout_kdl(multi, url, &jit_config()).unwrap();
        let out_single = split_preserving_layout_kdl(single, url, &jit_config()).unwrap();
        assert_eq!(out_single, out_multi);
    }

    #[test]
    fn regenerating_from_the_transforms_own_output_is_a_fixed_point() {
        // Fixture note: this uses the multi-line chrome form; zellij's real
        // dump serializes chrome single-line
        // (single_line_chrome_regenerates_to_a_fixed_point covers that path).
        // A tab wearing the transform's own output can be dumped and
        // rebuilt again mid-session — a later toggle, or a remote election
        // (F4) retrofitting a tab that already carries another instance's
        // rail. The dump then shows exactly the E1/E2 costumed shape: a
        // genuine rail pane plus shells wearing rail-like borderless/size
        // dressing from earlier regenerations. Regenerating from that shape
        // must reproduce the same docked body — any accretion here
        // compounds on every toggle.
        let dump = r#"layout {
    tab name="Tab #3" focus=true hide_floating_panes=true {
        pane size=1 borderless=true {
            plugin location="zellij:tab-bar"
        }
        pane split_direction="vertical" {
            pane size=28 borderless=true name="sidebar" {
                plugin location="file:/tmp/zellij-sidebar.wasm" {
                    rail "1"
                }
            }
            pane size=28 borderless=true {
                pane cwd="/a" size="33%"
                pane borderless=true cwd="/b" size="33%"
                pane borderless=true cwd="/c" size="34%"
            }
        }
        pane size=1 borderless=true {
            plugin location="zellij:status-bar"
        }
    }
}
"#;
        let url = "file:/tmp/zellij-sidebar.wasm";
        let config = jit_config();
        let first = split_preserving_layout_kdl(dump, url, &config).unwrap();
        let docked1 = docked_tab_body(&first);

        // Exactly one rail slot, and only chrome/rail borderless, in the
        // first generation already.
        assert_eq!(docked1.iter().filter(|l| l.contains(url)).count(), 1);
        assert_eq!(
            docked1.iter().filter(|l| l.contains("borderless")).count(),
            3 // tab-bar + rail + status-bar
        );

        let redump = format!("layout {{\n tab {{\n{}\n }}\n}}\n", docked1.join("\n"));
        let second = split_preserving_layout_kdl(&redump, url, &config).unwrap();
        let docked2 = docked_tab_body(&second);

        assert_eq!(docked2.iter().filter(|l| l.contains(url)).count(), 1);
        assert_eq!(
            docked2.iter().filter(|l| l.contains("borderless")).count(),
            3
        );
        assert_eq!(normalize(&docked1), normalize(&docked2));
    }

    #[test]
    fn chromeless_tabs_regenerate_without_inventing_chrome() {
        // Swaps must carry exactly the chrome the tab actually has — adding
        // chrome the tab lacks leaves the swap unfittable (swaps never
        // spawn) and wedges the tab.
        let dump = r#"layout {
    tab name="bare" {
        pane cwd="/only" size="50%"
        pane cwd="/other" size="50%"
    }
}
"#;
        let kdl = split_preserving_layout_kdl(dump, "file:/x.wasm", &jit_config()).unwrap();
        assert!(!kdl.contains("zellij:"));
        assert_eq!(kdl.matches("cwd=\"/only\"").count(), 2);
    }

    #[test]
    fn stray_rail_pane_in_the_region_is_dropped_not_reemitted() {
        // A prior retrofit's rail can ride along in the dump as a genuine
        // plugin pane (not just borderless dressing) — e.g. seated in a
        // sibling's slot after an absorb. rail_pane_kdl always contributes
        // the canonical slot, so any copy found in the region must be
        // dropped outright, not folded in as a second sidebar pane.
        let dump = r#"layout {
    tab name="Tab #1" {
        pane size=1 borderless=true {
            plugin location="zellij:tab-bar"
        }
        pane split_direction="vertical" {
            pane size=28 borderless=true name="sidebar" {
                plugin location="file:/tmp/zellij-sidebar.wasm" {
                    rail "1"
                }
            }
            pane cwd="/a" size="72%"
        }
        pane size=1 borderless=true {
            plugin location="zellij:status-bar"
        }
    }
}
"#;
        let kdl = split_preserving_layout_kdl(dump, "file:/tmp/zellij-sidebar.wasm", &jit_config())
            .unwrap();
        // Exactly one rail slot per tab body (docked, undocked, base) —
        // contributed solely by rail_pane_kdl.
        assert_eq!(
            kdl.matches("plugin location=\"file:/tmp/zellij-sidebar.wasm\"")
                .count(),
            3
        );
        assert_eq!(kdl.matches("rail \"1\"").count(), 3);
        assert_eq!(kdl.matches("cwd=\"/a\"").count(), 2);
    }

    #[test]
    fn stray_rail_is_recognized_defensively_by_location_substring() {
        // The exact-URL match cannot be the only net: a differently
        // configured or differently pathed copy of the sidebar plugin must
        // still be recognized and dropped.
        let dump = r#"layout {
    tab name="t" {
        pane split_direction="vertical" {
            pane size=28 borderless=true {
                plugin location="file:/other/path/zellij-sidebar.wasm" {
                    rail "2"
                }
            }
            pane cwd="/shell"
        }
    }
}
"#;
        let kdl = split_preserving_layout_kdl(dump, "file:/tmp/zellij-sidebar.wasm", &jit_config())
            .unwrap();
        assert!(!kdl.contains("/other/path/zellij-sidebar.wasm"));
        assert!(!kdl.contains("rail \"2\""));
        assert_eq!(
            kdl.matches("plugin location=\"file:/tmp/zellij-sidebar.wasm\"")
                .count(),
            3
        );
        assert_eq!(kdl.matches("cwd=\"/shell\"").count(), 2);
    }

    #[test]
    fn dump_with_a_tiled_sidebar_is_recognized_as_already_retrofitted() {
        let url = "file:/tmp/zellij-sidebar.wasm";
        let retrofitted = r#"layout {
    tab name="t" {
        pane split_direction="vertical" {
            pane size=28 borderless=true name="sidebar" {
                plugin location="file:/tmp/zellij-sidebar.wasm" {
                    rail "1"
                }
            }
            pane cwd="/a"
        }
    }
}
"#;
        assert!(dump_contains_sidebar(retrofitted, url), "resident rail detected");
        let other_path = retrofitted.replace("/tmp/zellij-sidebar.wasm", "/other/zellij-sidebar.wasm");
        assert!(
            dump_contains_sidebar(&other_path, url),
            "a differently-pathed sidebar is still recognized"
        );
        assert!(
            !dump_contains_sidebar(dump_fixture(), url),
            "a never-retrofitted tab of shells and chrome carries no sidebar"
        );
        let floating_only = r#"layout {
    tab name="t" {
        pane cwd="/a"
        floating_panes {
            pane {
                plugin location="file:/tmp/zellij-sidebar.wasm" {
                    rail "1"
                }
            }
        }
    }
}
"#;
        assert!(
            !dump_contains_sidebar(floating_only, url),
            "a floating sidebar does not own the tab's tiled region"
        );
    }

    #[test]
    fn dump_with_a_single_line_rail_is_recognized_as_already_retrofitted() {
        // The server strips only the requesting instance's own rail from a
        // dump, and a surviving resident's rail serializes on ONE line. The
        // seeder-abort must see it, or a second instance fires a double
        // override on an already-docked tab.
        assert!(
            dump_contains_sidebar(single_line_rail_dump(), "file:/tmp/zellij-sidebar.wasm"),
            "single-line resident rail detected"
        );
    }

    #[test]
    fn third_party_plugin_pane_survives_extraction() {
        // A user's non-builtin, non-sidebar plugin pane must ride along into
        // both swaps: dropping it would delete the pane from the swap story
        // (swaps never spawn) and orphan it on application.
        let dump = r#"layout {
    tab name="t" {
        pane size=1 borderless=true {
            plugin location="zellij:tab-bar"
        }
        pane cwd="/shell" size="60%"
        pane size="40%" {
            plugin location="file:/other/statusdeck.wasm"
        }
        pane size=1 borderless=true {
            plugin location="zellij:status-bar"
        }
    }
}
"#;
        let kdl = split_preserving_layout_kdl(dump, "file:/tmp/zellij-sidebar.wasm", &jit_config())
            .unwrap();
        assert_eq!(
            kdl.matches("plugin location=\"file:/other/statusdeck.wasm\"")
                .count(),
            2
        );
        assert_eq!(kdl.matches("cwd=\"/shell\"").count(), 2);
    }

    #[test]
    fn builtin_strider_region_pane_survives_extraction() {
        // zellij:* is not synonymous with chrome: strider is a 20%-wide
        // region pane in the builtin strider layout (`zellij setup
        // --dump-layout strider`), serialized single-line in dumps like all
        // plugin panes. Treating it as chrome re-emits the user's file
        // browser as a size=1 bottom bar; it must ride along into both
        // swaps like a third-party pane.
        let dump = r#"layout {
    tab name="t" focus=true hide_floating_panes=true {
        pane size=1 borderless=true { plugin location="zellij:tab-bar" }
        pane split_direction="vertical" {
            pane size="20%" { plugin location="zellij:strider" }
            pane cwd="/shell" size="80%"
        }
        pane size=2 borderless=true { plugin location="zellij:status-bar" }
    }
}
"#;
        let kdl = split_preserving_layout_kdl(dump, "file:/tmp/zellij-sidebar.wasm", &jit_config())
            .unwrap();
        assert_eq!(
            kdl.matches("plugin location=\"zellij:strider\"").count(),
            2,
            "strider rides along into both swap bodies, never the base template"
        );
        assert!(
            !kdl.contains("size=1 borderless=true { plugin location=\"zellij:strider\""),
            "strider is not re-emitted as a chrome row"
        );
        assert_eq!(kdl.matches("zellij:tab-bar").count(), 3);
        assert_eq!(kdl.matches("zellij:status-bar").count(), 3);
    }

    #[test]
    fn borderless_costume_is_stripped_from_region_panes() {
        // Observed live: repeated toggles left ordinary shells and
        // containers costumed with borderless=true from a stale rail slot.
        // User panes are never borderless; the only rows that legitimately
        // carry it are chrome and the rail, and those are regenerated from
        // scratch regardless of what the dump says. Sizes are left intact:
        // guessing them wrong corrupts real layouts, while a stray fixed
        // size is merely ugly and user-correctable.
        let dump = r#"layout {
    tab name="Tab #3" {
        pane size=1 borderless=true {
            plugin location="zellij:tab-bar"
        }
        pane cwd="/a" size="50%" borderless=true
        pane size="50%" borderless=true {
            pane cwd="/b" size="60%"
            pane cwd="/c" size="40%" borderless=true
        }
        pane size=1 borderless=true {
            plugin location="zellij:status-bar"
        }
    }
}
"#;
        let kdl = split_preserving_layout_kdl(dump, "file:/x.wasm", &jit_config()).unwrap();
        // Sizes survive untouched at every level; only borderless is gone.
        assert_eq!(kdl.matches("cwd=\"/a\" size=\"50%\"").count(), 2);
        assert_eq!(kdl.matches("pane size=\"50%\" {").count(), 2);
        assert_eq!(kdl.matches("cwd=\"/b\" size=\"60%\"").count(), 2);
        assert_eq!(kdl.matches("cwd=\"/c\" size=\"40%\"").count(), 2);
        // Only the canonical chrome (2 rows) and rail (1 row) per tab body
        // carry borderless — none of it belongs to the region.
        assert_eq!(kdl.matches("borderless=true").count(), 9);
    }

    #[test]
    fn quoted_strings_do_not_confuse_structure_or_attribute_stripping() {
        // Titles and cwds may contain braces and attr-lookalike text; only
        // real properties outside strings may be touched.
        let dump = r#"layout {
    tab name="Tab {5}" focus=true {
        pane size=1 borderless=true {
            plugin location="zellij:tab-bar"
        }
        pane name="build {debug}" cwd="/tmp/name=weird{dir" focus=true size="70%"
        pane cwd="/plain" size="28%"
        pane size=1 borderless=true {
            plugin location="zellij:status-bar"
        }
    }
}
"#;
        let kdl = split_preserving_layout_kdl(dump, "file:/x.wasm", &jit_config()).unwrap();
        assert!(!kdl.contains("build {debug}"));
        assert!(!kdl.contains("focus=true"));
        assert_eq!(kdl.matches("cwd=\"/tmp/name=weird{dir\"").count(), 2);
        assert_eq!(kdl.matches("cwd=\"/plain\"").count(), 2);
        // Structure survived: three tab bodies, all balanced.
        assert_eq!(kdl.matches("zellij:status-bar").count(), 3);
        assert_eq!(
            kdl.matches('{').count(),
            kdl.matches('}').count() + kdl.matches("weird{dir").count()
        );
    }

}
