//! The state store is the contract: `agent-status` writes one JSON file
//! per tmux session, this reads them. Writing is the CLI's alone, lock
//! and all — so the transitions only a running renderer can notice (a
//! Claude that died with its tmux session still up, a `done` the user
//! has now looked at, a session that has gone away) are asked for
//! rather than made. Slots are the CLI's too; nothing here assigns one.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Instant;

use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use serde::Deserialize;

use crate::config::{self, State};
use crate::tmux::{self, Panes};

/// One store entry, as much of it as the display needs.
#[derive(Deserialize)]
pub struct Entry {
    pub tmux_session: Option<String>,
    pub state: Option<String>,
    pub slot: Option<u32>,
    pub label: Option<String>,
}

/// A session showing `done`, kept so focus can demote it.
pub struct Done {
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

    fn find(&self, slot: u32) -> Option<&Tracked> {
        self.tracked.iter().find(|t| t.slot == slot)
    }
}

/// Read the store: slot states for the board, plus the housekeeping that
/// falls out of it. A tmux session that no longer exists is dropped
/// silently (killing a scratch session isn't a crash); one that outlived
/// its Claude shows as an error.
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
        let Some(entry) = read_entry(&path) else {
            continue;
        };
        let session = entry.tmux_session.clone();
        let health = match &session {
            Some(s) => health.of(s),
            None => Alive::WithClaude,
        };
        if health == Alive::No {
            if let Some(session) = &session {
                ask(&["drop", session]);
            }
            continue;
        }

        let mut state = entry.state.as_deref().and_then(State::parse);
        if let (Some(State::Done), Some(session)) = (state, &session) {
            snap.done.push(Done {
                session: session.clone(),
            });
        }
        // tmux alive but no Claude left: an unclean death, unless the entry
        // is parked `off` (a clean exit keeps the slot bound to the session).
        // Shown at once and persisted by the CLI, so the switcher and
        // `agent-status list` agree with the LEDs.
        if health == Alive::NoClaude && state != Some(State::Off) {
            state = Some(State::Error);
            if entry.state.as_deref() != Some(State::Error.as_str())
                && let Some(session) = &session
            {
                ask(&["set", session, State::Error.as_str()]);
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

/// `done` is sticky until the user is looking at the session. The CLI
/// re-reads before it writes, so a session a hook has moved on since is
/// left where it is.
pub fn demote_done_on_focus(done: &[Done]) {
    if done.is_empty() {
        return;
    }
    let focused = tmux::focused_sessions();
    for entry in done.iter().filter(|d| focused.contains(&d.session)) {
        ask(&["demote", &entry.session]);
    }
}

/// Ask the status CLI to write. Off-thread: it's python, and a paint
/// shouldn't wait on an interpreter starting. Every one of these is
/// idempotent, so the worst a lost race costs is asking twice.
fn ask(args: &[&str]) {
    let args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
    thread::spawn(
        move || match Command::new("agent-status").args(&args).status() {
            Ok(status) if status.success() => {}
            Ok(status) => crate::log(&format!(
                "agent-status {} failed ({status})",
                args.join(" ")
            )),
            Err(e) => crate::log(&format!("could not run agent-status ({e})")),
        },
    );
}

fn read_entry(path: &Path) -> Option<Entry> {
    serde_json::from_str(&fs::read_to_string(path).ok()?).ok()
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
    fn sparse_entries_parse() {
        let entry: Entry = serde_json::from_str(r#"{"tmux_session":"s","slot":1}"#).unwrap();
        assert_eq!(entry.state, None);
        assert_eq!(entry.label, None);
        // the CLI tracks more than this per entry; anything we don't read
        // has to be ignored rather than refused
        let rich: Entry = serde_json::from_str(
            r#"{"tmux_session":"s","slot":1,"state":"done","label":"l","ts":1,
                "claudes":{"a":{"state":"done","pane_id":"%1"}}}"#,
        )
        .unwrap();
        assert_eq!(rich.state.as_deref(), Some("done"));
    }
}
