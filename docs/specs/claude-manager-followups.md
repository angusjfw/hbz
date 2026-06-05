# Claude manager follow-ups

Ongoing list of known issues and pending work for the `claude-manager`
skill and its `-wrap` / `-shutdown` / `-end` siblings. Companions to
`2026-04-29-claude-manager-workflow.md` and the
`2026-05-22-claude-manager-sessions-pivot.md` follow-up spec. One
heading per item, terse context only — fixes and scoping decided when
picked up. Items below are ordered by priority, high to low.

## Asymmetric worker/manager wrap roles

Worker self-wrap runs steps 1–5 including the kill; manager-wrap runs
journal write + remove. Wrap state can drift between the two views.
Candidate fix: shift fully to the `wrap_requested` marker flow so the
manager owns the kill and the asymmetry collapses.

## Resumed_session_id vs resume_state overlap

Post sessions pivot, the registry's `resumed_session_id` field
duplicates the primary worker's `claude_session_id` in the resume_state
file. Kept by design so the registry alone is enough to fire a manual
`claude --resume` for the common single-worker case; the resume_state
file is authoritative for full cold resume. Worth revisiting if the
field grows confusing.

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
