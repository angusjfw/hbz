//! The state store is the contract: `agent-status` writes one JSON file
//! per tmux session, this reads them. Housekeeping lives here too — GC of
//! dead sessions, the error state and done-demotion on focus. Slots are
//! the CLI's alone; nothing here assigns or reserves one.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::config::{self, State};
use crate::tmux::{self, Panes};

/// One store entry. `state` is the aggregate the status CLI computes from
/// the entry's per-Claude states; everything else it tracks (`claudes`,
/// ids) rides along in `rest` so rewrites here never drop it.
#[derive(Deserialize, Serialize)]
pub struct Entry {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tmux_session: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slot: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ts: Option<i64>,
    #[serde(flatten)]
    rest: Map<String, Value>,
}

/// A session showing `done`, kept so focus can demote it.
pub struct Done {
    path: PathBuf,
    session: String,
}

/// One tracked session, as the board and the HUD see it.
pub struct Tracked {
    pub slot: u32,
    pub state: State,
    pub label: String,
    pub session: Option<String>,
}

#[derive(Default)]
pub struct Snapshot {
    /// Every entry holding a slot, dark ones included: `off` shows greyed
    /// in the HUD, and pressing a parked key still switches to its session.
    pub tracked: Vec<Tracked>,
    pub done: Vec<Done>,
}

impl Snapshot {
    /// Slots the board should light.
    pub fn lit(&self) -> BTreeMap<u32, State> {
        self.tracked
            .iter()
            .filter(|t| t.state.color().is_some())
            .map(|t| (t.slot, t.state))
            .collect()
    }

    pub fn label(&self, slot: u32) -> Option<&str> {
        self.find(slot).map(|t| t.label.as_str())
    }

    pub fn session(&self, slot: u32) -> Option<&str> {
        self.find(slot)?.session.as_deref()
    }

    fn find(&self, slot: u32) -> Option<&Tracked> {
        self.tracked.iter().find(|t| t.slot == slot)
    }
}

/// Read the store: slot states for the board, plus the housekeeping that
/// falls out of it. A tmux session that no longer exists is GC'd silently
/// (killing a scratch session isn't a crash); one that outlived its Claude
/// shows as an error.
pub fn read(health: &mut Health) -> Snapshot {
    let mut snap = Snapshot::default();
    let Ok(dir) = fs::read_dir(config::state_dir()) else {
        return snap;
    };
    for file in dir.flatten() {
        let path = file.path();
        if path.extension().is_none_or(|e| e != "json") {
            continue;
        }
        // the CLI keeps its slot memory in here too; that's its business
        if path.file_name().is_some_and(|name| name == "slots.json") {
            continue;
        }
        let Some(mut entry) = read_entry(&path) else {
            continue;
        };
        let session = entry.tmux_session.clone();
        let health = match &session {
            Some(s) => health.of(s),
            None => Alive::WithClaude,
        };
        if health == Alive::No {
            let _ = fs::remove_file(&path);
            continue;
        }

        let mut state = entry.state.as_deref().and_then(State::parse);
        if let (Some(State::Done), Some(session)) = (state, &session) {
            snap.done.push(Done {
                path: path.clone(),
                session: session.clone(),
            });
        }
        // tmux alive but no Claude left: an unclean death, unless the entry
        // is parked `off` (a clean exit keeps the slot bound to the session).
        // Persisted, so the HUD and `agent-status list` agree with the LEDs.
        if health == Alive::NoClaude && state != Some(State::Off) {
            state = Some(State::Error);
            if entry.state.as_deref() != Some(State::Error.as_str()) {
                entry.state = Some(State::Error.as_str().to_string());
                entry.ts = Some(now_ts());
                write_entry(&path, &entry);
            }
        }

        let Some(slot) = entry.slot.filter(|s| (1..=config::MAX_SLOTS).contains(s)) else {
            continue;
        };
        let label = entry
            .label
            .filter(|l| !l.is_empty())
            .or_else(|| session.clone())
            .unwrap_or_else(|| format!("slot {slot}"));
        snap.tracked.push(Tracked {
            slot,
            // an unreadable or missing state is as good as parked
            state: state.unwrap_or(State::Off),
            label,
            session,
        });
    }
    // tmux creation order, so the HUD reads in the switcher's order
    let order = tmux::creation_order();
    snap.tracked.sort_by_key(|t| {
        let by_session = t
            .session
            .as_ref()
            .and_then(|s| order.get(s).copied())
            .unwrap_or(u32::MAX);
        (by_session, t.slot)
    });
    health.prune();
    snap
}

/// `done` is sticky until the user is looking at the session.
pub fn demote_done_on_focus(done: &[Done]) {
    if done.is_empty() {
        return;
    }
    let focused = tmux::focused_sessions();
    for entry in done.iter().filter(|d| focused.contains(&d.session)) {
        // re-read rather than rewrite the snapshot: a hook may have moved
        // the session on since, and only a still-done entry demotes
        let Some(mut fresh) = read_entry(&entry.path) else {
            continue;
        };
        if fresh.state.as_deref() != Some(State::Done.as_str()) {
            continue;
        }
        demote(&mut fresh);
        write_entry(&entry.path, &fresh);
    }
}

/// `done` back to `idle`, per-Claude states included — the top-level state
/// is only their aggregate, so leaving a sub-state at `done` would have the
/// next event from a sibling Claude revive it.
fn demote(entry: &mut Entry) {
    entry.state = Some(State::Idle.as_str().to_string());
    entry.ts = Some(now_ts());
    let Some(claudes) = entry.rest.get_mut("claudes").and_then(Value::as_object_mut) else {
        return;
    };
    for claude in claudes.values_mut() {
        if let Some(claude) = claude.as_object_mut()
            && claude.get("state").and_then(Value::as_str) == Some(State::Done.as_str())
        {
            claude.insert("state".to_string(), Value::from(State::Idle.as_str()));
        }
    }
}

fn read_entry(path: &Path) -> Option<Entry> {
    serde_json::from_str(&fs::read_to_string(path).ok()?).ok()
}

fn write_entry(path: &Path, entry: &Entry) {
    let Ok(json) = serde_json::to_string(entry) else {
        return;
    };
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".tmp");
    let tmp = path.with_file_name(name);
    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }
    if fs::write(&tmp, json).is_ok() {
        let _ = fs::rename(&tmp, path);
    }
}

fn now_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or_default()
}

/// Whether a tmux session is around, and whether Claude is still in it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Alive {
    No,
    WithClaude,
    NoClaude,
}

/// Per-session liveness, cached: every probe costs a tmux invocation.
pub struct Health {
    cache: HashMap<String, (Alive, Instant)>,
}

impl Health {
    pub fn new() -> Health {
        Health {
            cache: HashMap::new(),
        }
    }

    pub fn of(&mut self, session: &str) -> Alive {
        if let Some((alive, at)) = self.cache.get(session)
            && at.elapsed() < config::HEALTH_TTL
        {
            return *alive;
        }
        let alive = probe(session);
        self.cache
            .insert(session.to_string(), (alive, Instant::now()));
        alive
    }

    /// Forget sessions nothing has asked about lately.
    fn prune(&mut self) {
        self.cache
            .retain(|_, (_, at)| at.elapsed() < config::HEALTH_TTL * 12);
    }
}

fn probe(session: &str) -> Alive {
    match tmux::pane_commands(session) {
        Panes::Missing => Alive::No,
        Panes::Commands(cmds) if cmds.iter().any(|c| tmux::looks_like_claude(c)) => {
            Alive::WithClaude
        }
        Panes::Commands(_) => Alive::NoClaude,
        // tmux itself wouldn't run: that's no evidence a session died, and
        // GC deletes state, so assume the best
        Panes::Unavailable => Alive::WithClaude,
    }
}

/// Watch the state dir. Store changes push, so paints and the HUD don't
/// wait on a poll cycle; the periodic re-read is only a backstop.
pub fn watch() -> (Receiver<()>, Option<RecommendedWatcher>) {
    let (tx, rx) = mpsc::channel();
    let dir = config::state_dir();
    let watcher = notify::recommended_watcher(move |res: notify::Result<_>| {
        if res.is_ok() {
            let _ = tx.send(());
        }
    })
    .and_then(|mut w| {
        w.watch(&dir, RecursiveMode::NonRecursive)?;
        Ok(w)
    });
    match watcher {
        Ok(w) => (rx, Some(w)),
        Err(e) => {
            crate::log(&format!("no store watcher ({e}), polling instead"));
            (rx, None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demotion_reaches_the_per_claude_states() {
        let mut entry: Entry = serde_json::from_str(
            r#"{"tmux_session":"s","state":"done","slot":1,
                "claudes":{"a":{"state":"done","pane_id":"%1"},
                           "b":{"state":"idle","pane_id":"%2"}}}"#,
        )
        .unwrap();
        demote(&mut entry);
        assert_eq!(entry.state.as_deref(), Some("idle"));
        let claudes = entry.rest["claudes"].as_object().unwrap();
        assert_eq!(claudes["a"]["state"], "idle", "the done one falls back");
        assert_eq!(claudes["b"]["state"], "idle");
        assert_eq!(claudes["a"]["pane_id"], "%1", "and keeps its pane");
    }

    #[test]
    fn sparse_entries_parse() {
        let entry: Entry = serde_json::from_str(r#"{"tmux_session":"s","slot":1}"#).unwrap();
        assert_eq!(entry.state, None);
        assert_eq!(entry.label, None);
        assert_eq!(
            serde_json::to_string(&entry).unwrap(),
            r#"{"tmux_session":"s","slot":1}"#
        );
    }
}
