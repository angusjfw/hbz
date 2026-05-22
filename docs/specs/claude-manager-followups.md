# Claude manager follow-ups

Ongoing list of known issues and pending work for the `claude-manager`
skill and its `-wrap` / `-shutdown` / `-end` siblings. Companions to
`2026-04-29-claude-manager-workflow.md` and the
`2026-05-22-claude-manager-sessions-pivot.md` follow-up spec. One
heading per item, terse context only — fixes and scoping decided when
picked up.

## Resume-replay of mid-wrap

A session killed mid-`/claude-manager-wrap` or `/claude-manager-shutdown`
and later resumed re-runs the wrap mechanics against stale state.
Repeatable today.

## Edit-tool blindness to stale file state

Wrap-time registry rewrites by workers sometimes succeed against an
in-memory view that no longer matches disk, or fail silently. Points at
needing a mandatory re-read under lock before write.

## Asymmetric worker/manager wrap roles

Worker self-wrap runs steps 1–5 including the kill; manager-wrap runs
journal write + remove. Wrap state can drift between the two views.
Candidate fix: shift fully to the `wrap_requested` marker flow so the
manager owns the kill and the asymmetry collapses.

## Over-prompting on spawn briefs

"Open a session to work on X" briefs should stay narrow. Captured
separately as user memory `feedback_dont_over_prompt`. Repo-side todo:
whether the skill itself should codify the expected brief format.

## hbz convention awareness in spawn flow

Spawning a worker in hbz should not instruct worktrunk / branches / PRs
(this repo commits direct to main). Captured separately as user memory
`hbz_repo_conventions`. Repo-side todo: enforce in the skill (per-repo
cwd detection) or via a hbz-side CLAUDE.md/AGENTS.md.

## Resumed_session_id vs resume_state overlap

Post sessions pivot, the registry's `resumed_session_id` field
duplicates the primary worker's `claude_session_id` in the resume_state
file. Kept by design so the registry alone is enough to fire a manual
`claude --resume` for the common single-worker case; the resume_state
file is authoritative for full cold resume. Worth revisiting if the
field grows confusing.
