# Claude manager follow-ups

Ongoing list of known issues and pending work for the `claude-manager`
skill and its `-wrap` / `-shutdown` / `-end` siblings. Companions to
`2026-04-29-claude-manager-workflow.md` and the
`2026-05-22-claude-manager-sessions-pivot.md` follow-up spec. One
heading per item, terse context only — fixes and scoping decided when
picked up. Items below are ordered by priority, high to low.

## Pane base-index assumption in cold resume

Cold resume implicitly assumes `pane-base-index 0` (and `base-index 0`).
Shutdown writes whatever tmux reports for `#{pane_index}` /
`#{window_index}`; cold resume splits "for each pane n>0", which
mis-handles non-zero base-index. Detect base-indexes at resume time or
rewrite the split rule as "(n-1) splits where n = panes recorded for
the window".

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

## Asymmetric worker/manager wrap roles

Worker self-wrap runs steps 1–5 including the kill; manager-wrap runs
journal write + remove. Wrap state can drift between the two views.
Candidate fix: shift fully to the `wrap_requested` marker flow so the
manager owns the kill and the asymmetry collapses.

## Claude pane detection specification

SKILL and FLOW say "by `pane_current_command` containing claude or
content sniff" without specifying the sniff. Should be concrete:
capture last ~30 lines and grep for `esc to interrupt` or a trailing
`> ` prompt before treating a pane as Claude.

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

## Vestigial preamble vars in FLOW.md

After the sessions pivot, the worker preamble still captures
`src_window` but no longer references it. Drop it.

## Manager cwd drifts when spawning

Spawning sessions and creating worktrees run `cd <path>` inside Bash
tool calls; the tool's working directory persists across calls, so the
manager's own cwd drifts away from where it started (recently ended up
in `off_the_job` after a spawn). Cosmetic — the manager's statusline
misreports its location mid-session, which is confusing. Mitigation:
prefer `git -C <path>`, absolute paths, and the `-c`/`-C` flags on
`wt`/`tmux` so the manager's own cwd stays put.
