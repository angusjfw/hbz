# Claude manager follow-ups

Ongoing list of known issues and pending work for the `claude-manager`
skill and its `-wrap` / `-shutdown` / `-end` siblings. Companions to
`2026-04-29-claude-manager-workflow.md` and the
`2026-05-22-claude-manager-sessions-pivot.md` follow-up spec. One
heading per item, terse context only — fixes and scoping decided when
picked up. Items below are ordered by priority, high to low.

## Wrap/shutdown drops the attached client out of tmux

The final step of both modes is `tmux kill-session -t "$src_session"`
(wrap step 5, shutdown step 6 in `claude-manager-end/FLOW.md`). The
user is attached to that session as a client, so the kill detaches
them to the parent shell instead of leaving them in tmux. Nothing
switches the attached client to another live session first. Candidate
fix: before the kill, if a client is attached to `$src_session`,
`tmux switch-client` it to the manager session (or any other live
session), then kill. Edge case: if it's the only session on the
server the kill always drops to the shell — nothing tmux can do.

## Worktree base staleness

`wt` branches off the LOCAL `master` / `main`, which is often stale
relative to `origin`. When the work depends on a recent merge, the
worker ends up on a base that doesn't contain it (hit today for
ASY-2522 needing ASY-2519). Spawn recipe should
`git -C <repo> fetch origin <default-branch>` first, then either
branch from `origin/<default-branch>` or `reset --hard` the wt branch
to that ref when recency matters.

## Resume-replay of mid-wrap

A session killed mid-`/claude-manager-wrap` or
`/claude-manager-shutdown` and later resumed re-runs the wrap mechanics
against stale state. Repeatable today.

## Cold-resume partial-failure recovery

If `tmux new-session` succeeds but a later `new-window` /
`split-window` / `select-layout` fails (worktree missing, layout
malformed), the tmux session is half-built and the registry stays in
`shutdown`. No documented rollback. Minimum: on failure,
`tmux kill-session -t <id>` and leave the registry in `shutdown` so the
user can retry.

## Edit-tool blindness to stale file state

Wrap-time registry rewrites by workers sometimes succeed against an
in-memory view that no longer matches disk, or fail silently. Points at
needing a mandatory re-read under lock before write.

## Wrap-fulfilment cadence

Manager should process `wrap_requested` markers as the watch surfaces
them (or at the next idle moment), not let them queue. Backlog of 16
hit today before any were processed. At minimum the manager should
surface the count of pending wraps periodically so the backlog stays
visible; the goal is to keep it at ≤1. Backlog is the upstream
driver of the wrap_requested-reopen case in "Reopening sessions from
disk" above.

## Manager shutdown flow

A single termination procedure for the manager: walk every live
registry entry, resolve each one with the user (shut down, wrap, or
reconcile), then exit. Currently no documented end-of-session flow —
entries linger pointing at tmux sessions that no longer exist after
the user ends the day or restarts the machine.

## Asymmetric worker/manager wrap roles

Worker self-wrap runs steps 1–5 including the kill; manager-wrap runs
journal write + remove. Wrap state can drift between the two views.
Candidate fix: shift fully to the `wrap_requested` marker flow so the
manager owns the kill and the asymmetry collapses.

## Multi-window wrap journal coverage

Shutdown captures every Claude pane's `claude_session_id` in
resume_state. Wrap only captures the calling worker's
`resumed_session_id`; forked workers in other windows don't appear in
the journal entry. Decide whether that's correct (the calling worker
is "the" worker) or whether the journal should record forks.

## External rename-session stale `tmux_session`

If the user runs `tmux rename-session` outside the manager, the
registry's `tmux_session` becomes stale. Reconcile catches it as
"session not found" and asks; could be tightened by matching the
registry id against `#{session_name}` across the server when the
recorded name doesn't exist.

## hbz convention awareness in spawn flow

Spawning a worker in hbz should not instruct worktrunk / branches / PRs
(this repo commits direct to main). Repo-side todo: enforce in the
skill (per-repo cwd detection) or via a hbz-side CLAUDE.md/AGENTS.md.

## Resumed_session_id vs resume_state overlap

Post sessions pivot, the registry's `resumed_session_id` field
duplicates the primary worker's `claude_session_id` in the resume_state
file. Kept by design so the registry alone is enough to fire a manual
`claude --resume` for the common single-worker case; the resume_state
file is authoritative for full cold resume. Worth revisiting if the
field grows confusing.

## Manager cwd drifts when spawning

Spawning sessions and creating worktrees run `cd <path>` inside Bash
tool calls; the tool's working directory persists across calls, so the
manager's own cwd drifts away from where it started. Cosmetic — the
manager's statusline misreports its location mid-session, which is
confusing. Working recipe: subshell the cd (`( cd <repo> && wt … )`)
so the parent shell's cwd is unaffected, or use `git -C <repo>` and
`wt -C <repo>` / `tmux -c <cwd>` flags. Never bare `cd` in the manager
shell.

## Paused state for sessions

A third lifecycle state between active and shutdown: paused. The tmux
session stays alive and the worker keeps its state; the manager just
flags it so it sorts to the bottom of the tmux leader+w switcher and
shows a paused marker alongside existing identifiers in the title.
Use case: sessions waiting on review or other external state — the
switcher should distinguish "what I'm working on now" from "what's
parked for later" without killing tmux. Distinct from the retired
park concept, which moved sessions out of the manager's window list;
this one just relabels and reorders.

## Watch is inert without consuming its output

Starting the registry watch (background Bash) is only half the
mechanism — the manager must wire a `Monitor` onto the watch's stdout
and treat each `changed:` line as the trigger for the reaction loop
(re-read registry, diff against last-known, surface worker-driven
changes, sync the task list, process any `wrap_requested`). Without
that consumption, the watch faithfully logs every worker write but the
manager never reacts: wraps and shutdowns are only discovered on a
manual registry re-read or when the user flags them. Observed: a full
manager session ran the watch the entire time but never Monitored it,
so every worker self-wrap went unnoticed until manual reconcile and a
16-deep wrap backlog built up. The on-invocation step "Start the
registry watch" should explicitly include attaching the Monitor, not
just spawning the background process — the two are useless apart.
