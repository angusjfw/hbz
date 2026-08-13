---
name: coordinator-worker
description: The engineering-coordination playbook for running several related threads of work in parallel as a coordinator, one worker Claude per task in its own tmux window, while you own the shared contract, the ordering, verification, and the quality gate. Use this when the user names the pattern — "act as a coordinator", "coordinate this build", "run this as a coordinator/worker build", "spawn a worker per ticket", "set up a worker team" — and equally when they don't name it but ask for its pieces: "create and coordinate two new tmux windows in this session", "spawn another window with claude working on X", "run these in parallel", or any set of several related tasks that each want their own durable session. If a request pairs "coordinate" with more than one new window or worker, this applies. The tasks need not be tickets or even code: a mixed team of code workers in worktrees, non-code workers on docs/tickets/comms, and plain tooling panes you drive yourself is the normal case, not an exception. Composes with claude-manager (which owns session lifecycle: spawn, tmux, registry, resume) rather than replacing it. NOT for a single task, a quick edit, or a one-off read-only subagent lookup.
---

# Coordinator/worker

A multi-task pattern: you are a **coordinator** running one **worker** Claude per task —
a task being whatever the unit actually is, a ticket, a repo, an ops job, a write-up — and
you never write product code yourself. Workers are durable peers, full sessions with their
own context, not fire-and-forget subagents. The point is that the work is long-lived,
steerable mid-flight, and survives across turns and resume, while your own context stays
clean because the workers' tool spam never enters it.

This is deliberately a *substantive* role. You review plans, own the shared contract and
the ordering, verify claims, and gate everything outward. That's more than the meta-only
`claude-manager` role does — see below.

## When this fits — and when it doesn't

- **This pattern** when there are several related tasks, they want parallel progress, and
  each needs its own durable session. Tier up: run the coordinator one model tier above
  the workers, so expensive tokens go on orchestration and the quality gate while cheaper
  workers do the grind.
- **A subagent** (not this) for a read-only sweep where you want a *conclusion*, not a
  process — lighter, and the right tool for that job.
- **Neither** for a single task or a quick edit — just do it, or spawn one worker without
  the coordination scaffold.

Don't require the tasks to share a code interface before you'll treat this as the pattern.
What binds them is whatever makes them wrong if handled independently, and that is just as
often **ordering** (one job has to land before the next starts) or **shared findings** (two
outputs have to describe the same reality) as a shared function signature. A DLQ drain, a
code change, and a project write-up about both is a coordinated set even though nothing in
it is a shared type.

Read the request for the *pieces*, not the name. "Create and coordinate two new tmux windows
in this session, one running our ops tooling and another with claude working on that PR — and
spawn another window for the project updates" is this pattern stated in full; it just never
says "coordinator". Take it as the trigger and set up properly, rather than opening three
windows ad hoc and skipping the contract.

Scale the ceremony, not the discipline. Three windows for half a day is the common size, and
at that size some of this is a no-op — merge order with one code branch, interface relay
between workers that don't share code. Four things still pay for themselves at any size,
because each is cheap now and unrecoverable later: the minted id and `worker:` line, a
written BRIEF, a stated boundary per worker, and verifying claims instead of taking them.

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

- One coordinator (top tier); one worker per task, each in its own tmux window. A worker
  that touches code gets **its own worktree off a fresh `origin/<default>`** (fetch first —
  local default branches go stale).
- Coordinator never writes product code. That is not a vow of inaction: your own hands-on
  investigation (logs, metrics, warehouse queries) and the ops panes you drive are yours to
  run, and usually should be, because they inform the calls only you can make. What you
  hand off is implementation.
- Workers never act outside the boundary you gave them — for a code worker that is its
  worktree, and for everyone else it has to be stated (see below).
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
- **The id has to survive your launcher.** Spawning through a worktree wrapper is where it
  gets quietly dropped, because the natural thing to type is the wrapper's own happy path
  (`wt switch --create <branch> -x claude`) and that starts a worker with no minted id. Put
  the whole command in the execute string, quoted —
  `wt switch --create <branch> -b origin/<default> -x 'claude --session-id <uuid> --effort high'`
  — because unquoted trailing flags get parsed by the wrapper, not passed to Claude. Then
  read the id back out of the pane's argv to confirm it took.

## Three kinds of window, and what each one needs

A real team is usually mixed. Decide which kind each window is *before* you open it, because
the isolation and the paperwork differ:

- **Code worker.** Its own worktree off fresh `origin/<default>`, its own DB and ports, a
  BRIEF, a STATUS file, a minted id and a `worker:` line. The pre-push review gate applies.
- **Non-code worker** — project updates, ticket writing, docs, a drafted message. Still a
  full peer session, still gets a BRIEF, a STATUS file, a minted id and a `worker:` line.
  What it does *not* get is a worktree, and that's the trap: a worktree is what silently
  keeps code workers off each other's files, and a non-code worker has no such fence. So
  **name the files and surfaces it owns, and the ones it must not touch.** Two workers with
  write access to one doc, or one editing a ticket you're also editing, clobber each other
  the same way two editors on a PR body do.
- **A tooling pane you drive yourself.** Ops tooling, a DLQ shovel, a prod shell, a tailing
  log. No Claude, so no BRIEF, no STATUS, no minted id and no `worker:` line — the registry
  line is for Claude panes only, and inventing one for a shell pane makes a resume try to
  restore a conversation that never existed. It still belongs in your spawned-resource
  inventory so teardown finds it, and its commands are yours: don't hand a destructive ops
  step to a worker to run on your behalf.

Non-code work is not a lesser member of the team. A write-up that has to describe what the
other workers actually did is *downstream of them*, which makes it exactly the thing that
goes stale if you let it start early or forget to feed it the verified outcome.

## The worker contract — a written BRIEF.md at spawn

Give each worker a file, not just a typed prompt:

- **The full task statement, inlined.** Ticket text, the relevant thread, the numbers you
  already established. Don't rely on the worker fetching it from a connector — those may
  need interactive auth the worker can't complete.
- **Hard guardrails:** local commits only; nothing pushed or posted without human
  approval; don't touch other panes, worktrees, or running servers; conventional commits
  with an `Assisted-by:` trailer.
- **Whatever binds this task to the others, stated explicitly.** The shared code interface
  ("pass the whole state object so the other task's addition flows through unchanged"); or
  the ordering ("this cannot start until the drain is confirmed clear"); or the shared facts
  ("these are the verified numbers — don't re-derive them, and flag it if yours disagree").
  This is the thing that goes wrong silently if left implicit, and it's the reason a worker
  needs you rather than just a prompt.
- **Its boundary** — the worktree, or for a non-code worker the explicit list of files and
  surfaces it owns.
- **The STATUS.md protocol** (append-only, one per worker):
  `PLAN` (then hold for your go) → `PROGRESS` / `QUESTION` (blocking, or non-blocking with
  a stated default) → `DONE` with verification evidence.
- **Absolute paths in every instruction** — workers resolve relative paths against their
  own worktree, so a relative path sends artifacts to the wrong place.

## Your duties as coordinator

- **Review each PLAN before any work starts.** Own the shared contract and the merge order.
  Relay changes between workers yourself — workers never message each other.
- **Own the ordering, and hold the downstream worker.** When one task depends on another
  finishing, the dependency lives with you: workers can't see each other, so a downstream
  worker left unblocked will happily produce a confident output built on a state that hasn't
  happened yet. Say what it's waiting for in its BRIEF, and release it yourself once you've
  verified the upstream result.
- **Watch STATUS files and panes with background watchers** so a worker landing its PLAN
  or a QUESTION wakes you between turns rather than waiting for your next manual check.
- **Independently verify worker claims** (git log/status, diffs, artifact files,
  screenshots) before acting on them. Reports and filesystem reality diverge — treat a
  worker's "done" as a claim to check, not a fact.
- **Gate everything outward on the human.** Batch product decisions into `AskUserQuestion`
  rounds rather than dripping them out. Write your reports for a reader who saw none of the
  session: lead with what needs them, then numbered, vetoable assumptions.

## Isolation and mechanics that bite

The first three are code-worker concerns; the last two apply to every pane.

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

Run an independent-perspective reviewer fan-out on each code branch *before* push — it's the
one step that reliably catches what a strong worker's own read misses. Then **read the
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

A non-code worker's output needs a gate too, and it's a different one: check every claim in
the write-up against what you verified yourself, not against what the workers reported. A
status update or ticket comment is outward-facing the moment it's posted, and it's the one
artifact that confidently states things nobody re-checked.

## If a heavyweight build harness is installed: durable bits only

Some harness plugins wrap the whole build lifecycle. Take the parts that add a perspective
you don't already have, and leave the parts that substitute for capability the workers have:

- **Use:** the review fan-out (above) as the pre-push gate; an a11y audit on changed UI;
  the close-out and doc-update steps at project end; the merge-and-cleanup step.
  Independent-perspective fan-outs and close-out discipline earn their place regardless of
  how strong the workers are.
- **Skip:** code-explorer agents, commit wrappers, and plan pipelines — scaffolding for
  weaker models. Strong workers explore inline and cut surgical commits natively, and your
  own plan gate covers planning. (Cheap optional: have workers drop their plans in a shared
  `plans/` dir so cross-session plan tooling still works.)

## Deeper reference

If the user keeps a local workspace wiki, its coordinator/worker workflow page holds the
worked examples and the running log of gotchas this playbook is distilled from — read it
for the accreted detail when a situation here is thinner than what you're facing.
