---
name: coordinator-worker
description: The engineering-coordination playbook for running a multi-ticket, multi-repo build as a coordinator with one worker Claude per ticket, each in its own worktree, while you own interfaces, merge order, verification, and the quality gate. Use this whenever the user says "act as a coordinator", "coordinate this build", "run this as a coordinator/worker build", "spawn a worker per ticket", "set up a worker team", or describes parallel work across several tickets or repos that share a cross-cutting interface — even if they don't name the pattern. Composes with claude-manager (which owns session lifecycle: spawn, tmux, registry, resume) rather than replacing it. NOT for a single-ticket task, a quick edit, or a one-off read-only subagent lookup.
---

# Coordinator/worker build

A multi-ticket build pattern: you are a **coordinator** orchestrating one **worker**
Claude per ticket, each in its own worktree, and you never write product code yourself.
Workers are durable peers — full sessions with their own context — not fire-and-forget
subagents. The point is that the work is long-lived, steerable mid-flight, and survives
across turns and resume, while your own context stays clean because the workers' tool
spam never enters it.

This is deliberately a *substantive* role. You review plans, own interfaces and merge
order, verify claims, and gate everything outward. That's more than the meta-only
`claude-manager` role does — see below.

## When this fits — and when it doesn't

- **This pattern** when the work is several tickets that share a cross-cutting interface,
  want parallel progress, and each needs its own durable session. Tier up: run the
  coordinator one model tier above the workers, so expensive tokens go on orchestration
  and the quality gate while cheaper workers do the grind.
- **A subagent** (not this) for a read-only sweep where you want a *conclusion*, not a
  process — lighter, and the right tool for that job.
- **Neither** for a single ticket or a quick edit — just do it, or spawn one worker
  without the coordination scaffold.

## Relationship to claude-manager

They look alike (both orchestrate workers in tmux) but they are different layers, and
you use both at once:

- **`claude-manager` owns session lifecycle** — spawning, the tmux containers, the
  session registry, shutdown/wrap/resume. It is deliberately *meta-only*: it never reads
  code or makes engineering calls. Use its mechanics for the plumbing of standing workers
  up and tearing them down.
- **This skill is what you do as the coordinator** — the engineering coordination on top:
  interfaces, plan gates, verification, the quality gate. You are typically yourself a
  `claude-manager`-spawned worker session that then runs *this* playbook.

Because of that layering, one rule matters most (below): keep your workers inside your own
tmux session so the manager still tracks the whole team as one unit.

## Shape

- One coordinator (top tier); one worker per ticket, each in **its own worktree off a
  fresh `origin/<default>`** (fetch first — local default branches go stale).
- Coordinator never writes product code; workers never act outside their own worktree.
- **Workers are panes/windows inside your own tmux session, never separate top-level tmux
  sessions.** Your session is the one `claude-manager` registry entry, so a single manager
  shutdown/wrap walks every pane, captures each worker's session id, and a cold resume
  rebuilds the whole team. A worker spawned as its own loose top-level session with no
  registry entry is orphaned from the manager — invisible to the registry, absent from any
  snapshot, not resumed with the team — while gaining nothing. If a worker genuinely needs
  an independent lifecycle, register it as its *own* `claude-manager` session instead of
  leaving it untracked.
- **Mint each worker's conversation id and record it, in the same action as spawning it.**
  Start the worker as `claude --session-id <uuid> …`, then add a `worker:` line to your own
  registry entry (`claude-manager` § Registry) with that id, the worker's cwd and a short
  label. Being inside your tmux session only protects the team against a *clean* shutdown,
  which walks the panes and reads their ids. It buys nothing against the case that actually
  loses work — the tmux server dying with no warning — because nothing outside the panes
  knows those conversations existed. The `worker:` line is what survives that, and writing
  it later means not writing it at all. Drop the line when the worker goes.

## The worker contract — a written BRIEF.md at spawn

Give each worker a file, not just a typed prompt:

- **Full ticket text.** Don't rely on the worker fetching it from a connector — those may
  need interactive auth the worker can't complete.
- **Hard guardrails:** local commits only; nothing pushed or posted without human
  approval; don't touch other panes, worktrees, or running servers; conventional commits
  with an `Assisted-by:` trailer.
- **The cross-ticket interface, stated explicitly** — e.g. "pass the whole state object so
  the other ticket's addition flows through unchanged." This is the thing that goes wrong
  silently if left implicit.
- **The STATUS.md protocol** (append-only, one per worker):
  `PLAN` (then hold for your go) → `PROGRESS` / `QUESTION` (blocking, or non-blocking with
  a stated default) → `DONE` with verification evidence.
- **Absolute paths in every instruction** — workers resolve relative paths against their
  own worktree, so a relative path sends artifacts to the wrong place.

## Your duties as coordinator

- **Review each PLAN before any code.** Own interfaces and merge order. Relay interface
  changes between workers yourself — workers never message each other.
- **Watch STATUS files and panes with background watchers** so a worker landing its PLAN
  or a QUESTION wakes you between turns rather than waiting for your next manual check.
- **Independently verify worker claims** (git log/status, diffs, artifact files,
  screenshots) before acting on them. Reports and filesystem reality diverge — treat a
  worker's "done" as a claim to check, not a fact.
- **Gate everything outward on the human.** Batch product decisions into `AskUserQuestion`
  rounds rather than dripping them out. Write your reports for a reader who saw none of the
  session: lead with what needs them, then numbered, vetoable assumptions.

## Isolation and mechanics that bite

- **Per-worker local DB and per-worker ports.** Shared DBs break delete-and-seed
  integration suites even without explicit resets. Never rebuild or restart anything a
  human is using — bring up an alternate-port stack (and an isolated build dir) instead.
- **Verify each worker's permission mode from its status line — don't assume it took.**
  Mode is cycled by keystroke, which is swallow-prone (worse when the TUI is in a vim
  insert mode), so a dropped or extra press lands a worker in a *different* mode than you
  chose. The permission mode is the hard gate; a BRIEF guardrail is only soft, so a worker
  one step too autonomous can act unattended. After setup, re-capture every worker's status
  line and reconcile actual mode to intent.
- **A fresh worktree's dependency tree is a liar.** Have each worker prove its toolchain
  resolves *locally* before trusting any green result. A copied virtualenv can keep its
  interpreter shebangs and `.pth` entries pointing at the original clone, so tests and type
  checks silently run against *other* code; a JS `node_modules` can be stale at creation
  and stale again after a rebase that bumps the lockfile. So: reinstall dependencies on
  worktree creation, and again after any rebase that touches a lockfile.
- **Tracked env files** (an `.env.test` that's committed, not gitignored): edit-then-revert,
  and check nothing leaks into a commit.
- **Verify every pane send actually landed** by re-capturing — messages to a TUI in vim
  mode get silently swallowed otherwise.

## Outward gates and shared-surface gotchas

- **Force-push classifiers refuse agent-relayed authority** (yours and the worker's alike).
  History rewrites need the human to name the push or run it. Plan for it, don't fight it.
- **Two editors on one surface clobber each other.** Editing a PR body from a stale local
  file has wiped human-attached images. Any shared-surface edit starts from the live state,
  or the surface belongs to exactly one owner.
- **PR images can't be uploaded with token auth** — humans drag-drop; agents stage the PNGs
  plus paste-ready markdown blocks.
- **No hard-wrapped prose in PR bodies** — it breaks long links. One line per
  paragraph/bullet.

## The quality gate: a pre-push review pass per branch

Run an independent-perspective reviewer fan-out on each branch *before* push — it's the one
step that reliably catches what a strong worker's own read misses. Then **read the
sub-threshold list, not just the pass/fail verdict** — the value is often in items that
scored just under the confirmation bar:

- **Ask whether each new test can actually fail.** A test can pass for the wrong reason
  (e.g. it builds fixtures in the same order the code happens to produce). The way to know
  is to delete the clause the test protects and see if it still passes — reading the test
  won't reveal it.
- **"Below the bar because reachability is unproven" is not "unreachable."** Hand it back
  for a reproduction rather than filing it away.
- **On performance items, ask for a measurement, not an argument.** A benchmark on
  prod-shaped data decides it; a plausibility argument doesn't.
- **Trust reviewers that correct their own premises mid-review** — treat that as a quality
  signal and prompt for it.
- **Give reviewers the moving-target caveat:** the branch may gain commits mid-review, so
  re-derive the review range from the merge base, never from `origin/<default>..HEAD`
  (which inflates once the default branch advances).

## harness-core: use the durable bits, skip the scaffolding

- **Use:** the review fan-out (above) as the pre-push gate; the a11y audit on changed UI;
  the close-out/doc-update steps at project end; the merge-and-cleanup step.
  Independent-perspective fan-outs and close-out discipline earn their place regardless of
  how strong the workers are.
- **Skip:** the code-explorer agent, the commit wrapper, and the plan pipeline — that's
  scaffolding for weaker models. Strong workers explore inline and cut surgical commits
  natively, and your own plan gate covers planning. (Cheap optional: have workers drop
  their plans into `plans/` so cross-session plan tooling still works.)

## Deeper reference

If the user keeps a local workspace wiki, its coordinator/worker workflow page holds the
worked examples and the running log of gotchas this playbook is distilled from — read it
for the accreted detail when a situation here is thinner than what you're facing.
