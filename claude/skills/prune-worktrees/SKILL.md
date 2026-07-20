---
name: prune-worktrees
description: >
  Use when the user says "prune worktrees", "clean up worktrees",
  "delete old/stale worktrees", "free disk space from old branches",
  "which worktrees can I delete", or "remove merged worktrees". Bulk-
  removes stale git worktree checkouts after cross-checking GitHub PR
  state. Discovers worktrees authoritatively from `git worktree list`
  (handles nested layouts and worktree managers like worktrunk),
  resolves each worktree's LIVE branch, gates deletion on
  `gh pr list`, refuses to touch dirty/stashed/open-PR/detached
  worktrees, and removes safe ones with the branch-cleaning primitive
  so parent-repo metadata is pruned in the same step.
user-invocable: true
allowed-tools: ["Bash", "Read", "Grep", "Glob", "AskUserQuestion"]
argument-hint: "[--workspace <dir>] [--protect <pattern,...>] [--dry-run] [--keep-closed]"
---

# Prune Worktrees

Bulk-remove stale git worktree checkouts once it is safe to do so.
Written for workspaces that accumulate dozens of worktrees — especially
when a manager (e.g. worktrunk / `wt`) copies gitignored deps such as
`node_modules` into every worktree, so each one costs gigabytes.

The core safety idea: **the directory name lies, the branch is fluid,
the only truth is `git status` + `git worktree list` + `gh pr list
--head <live-branch>` at removal time.** Every safeguard below exists
because at least one of those signals was wrong in a real run.

## Arguments

Parse `$ARGUMENTS`:

| Flag | Effect |
|------|--------|
| `--workspace <dir>` | Root to scan for main repos. Defaults to `$PWD`. |
| `--protect <patterns>` | Comma-separated ticket IDs or globs never to delete (e.g. `ASY-2907,dependabot-*`). Matched case-insensitively against both live branch and directory basename. |
| `--dry-run` | Survey + categorize + print the plan. Never remove anything. **Always run this first on a workspace you haven't pruned before.** |
| `--keep-closed` | Treat CLOSED-unmerged PRs as KEEP. Default: CLOSED-unmerged and MERGED are both delete candidates. |

If no protect list is given but the surrounding conversation implies
one (tickets/branches the user is mid-work on), restate it and confirm
with `AskUserQuestion` before proceeding.

---

## Hard Rules (non-negotiable)

1. **Never trust the directory name.** Resolve the live branch with
   `git -C <wt> branch --show-current` before any PR lookup or delete.
   Directory names are stale aliases; a dir named `foo-123` is
   routinely checked out on an unrelated branch.

2. **Never trust an agent's inventory.** Start from `git -C <repo>
   worktree list --porcelain` (authoritative) and reconcile against
   `ls`. Act only on paths present in the git worktree list.

3. **Gate deletion on `gh pr list`, not on ancestry or ticket state.**
   Orgs that **squash-merge** leave the branch commits as non-ancestors
   of the default branch, so `git branch --merged` / `--is-ancestor`
   report merged branches as unmerged — a false "keep". Linear/Jira
   state is advisory only (a "Todo" ticket can have a merged PR). The
   deletion gate is the PR `state` from GitHub.

4. **A `gh` failure is never "deletable".** No GitHub remote, auth
   error, or empty result → REVIEW, never delete. (Local-only repos
   such as a personal workspace meta-repo have no remote by design.)

5. **Never delete dirty or stashed work** unless the user names the
   specific path and accepts the loss. `git status --porcelain` must be
   empty. Note that stashes are **shared across a repo's worktrees** —
   check `git stash list` once at the main repo; treat non-empty as
   "needs review", not a blanket veto, after explaining it.

6. **Never touch these, ever:**
   - the main checkout of any repo (not a linked worktree);
   - a repo with **no GitHub remote** (local-only / meta-repos);
   - harness-managed worktrees under `**/.claude/worktrees/**`;
   - the worktree the skill is currently running in
     (`git rev-parse --show-toplevel`);
   - anything matching the protect list (branch OR dir basename).

7. **Detached HEAD → REVIEW.** Empty `branch --show-current` means
   `gh pr list --head ""` would match *every* PR — a huge false
   positive. Detect empty branch and bucket as REVIEW-DETACHED.

8. **Never `rm -rf` a registered worktree.** Remove via the manager or
   git so metadata is pruned atomically (see Phase 4). `rm -rf` orphans
   `.git/worktrees/<name>/` and breaks future worktree ops.

9. **Confirm at every tier boundary** with `AskUserQuestion`, showing
   PR numbers + states. The user can skip any tier.

---

## Phase 0: Preflight & manager detection

```bash
WS="${ARG_WORKSPACE:-$PWD}"
GIT=$(command -v git); GH=$(command -v gh)
[ -z "$GH" ] && echo "gh CLI required for PR verification" && exit 1
[ ! -d "$WS" ] && echo "workspace $WS not found" && exit 1
WT=$(command -v wt)   # worktrunk, if present — preferred remover (cleans branch)
CURRENT=$($GIT rev-parse --show-toplevel 2>/dev/null)   # never remove this
df -h "$WS" | tail -1
```

`du -sh` over gigabytes of `node_modules` is the slowest step by far —
across dozens of worktrees it dominates the run. It is **not needed to
decide anything**: the reclaimed figure comes from the `df` delta in
Phase 5. So skip per-worktree sizing during categorization; if the user
wants per-item sizes, `du` only the delete candidates after Phase 3, in
one pass, and expect it to be slow.

---

## Phase 1: Authoritative inventory

Find main repos, then let git enumerate their worktrees. Do **not**
assume worktrees are direct children of the workspace — managers nest
them (e.g. `<ws>/trees/<repo>/<branch>`), and `git worktree list`
reports the real paths regardless.

```bash
# candidate main repos: top-level dirs that are inside a work tree
for d in "$WS"/*/; do
  d="${d%/}"
  $GIT -C "$d" rev-parse --is-inside-work-tree >/dev/null 2>&1 || continue
  # skip repos with no GitHub remote (local-only / meta-repos)
  $GIT -C "$d" remote get-url origin 2>/dev/null | grep -qi github || continue
  MAIN=$($GIT -C "$d" rev-parse --show-toplevel)
  echo "$MAIN"
done | sort -u
```

For each unique MAIN, enumerate worktrees:

```bash
$GIT -C "$MAIN" worktree list --porcelain
```

Parse records (`worktree <path>` / `branch refs/heads/<name>` /
`detached` / blank line separates records). Exclude, per Hard Rule 6:
the MAIN path itself, `*/.claude/worktrees/*`, and `$CURRENT`.

Reconcile: a path git lists but missing on disk → schedule
`git worktree prune` (Tier 1). A dir on disk that git does not list →
not a worktree of this repo; leave it alone.

---

## Phase 2: Per-worktree probe

For each linked worktree:

```bash
BRANCH=$($GIT -C "$W" branch --show-current 2>/dev/null)
DIRTY=$($GIT -C "$W" status --porcelain 2>/dev/null | wc -l | tr -d ' ')
# do NOT du here — see Phase 0. Size only delete candidates later, if asked.
```

Then PR state, **queried by the live branch, from inside the worktree**
(note: `gh` has no `-C` flag — `cd` into the worktree so it uses that
worktree's origin remote):

```bash
if [ -n "$BRANCH" ]; then
  PR=$(cd "$W" && "$GH" pr list --head "$BRANCH" --state all \
        --json number,state --limit 1 \
        --template '{{if .}}{{(index . 0).number}} {{(index . 0).state}}{{else}}- NONE{{end}}' 2>/dev/null)
  # empty PR => gh error (no remote/auth) => treat as ERR, bucket REVIEW
fi
```

Check the shared stash once per MAIN (`git -C "$MAIN" stash list`).

---

## Phase 3: Categorize

Exactly one bucket per worktree:

| Bucket | Conditions |
|---|---|
| PROTECTED | matches `--protect` (branch OR dir basename) |
| KEEP-DIRTY | `DIRTY > 0` |
| KEEP-OPEN | PR `OPEN` |
| REVIEW-DETACHED | empty live branch (detached HEAD) |
| REVIEW-NOPR | live branch present but no PR ever opened |
| REVIEW | `gh` error / no remote / anything ambiguous |
| DELETE-MERGED | PR `MERGED`, clean, not protected |
| DELETE-CLOSED | PR `CLOSED` unmerged, clean, not protected, unless `--keep-closed` |

Print a table (path, live branch, PR state+number, dirty, bucket) with
bucket counts at the top. Add sizes only if the user asked for them
(du'd from the delete candidates alone).

---

## Phase 4: Tiered execution (skip entirely under `--dry-run`)

Confirm before each tier with `AskUserQuestion`.

**Removal primitive** — prefer the manager so the merged branch is
cleaned too:

```bash
if [ -n "$WT" ]; then
  # worktrunk: removes worktree + deletes branch IF merged (keeps it if not).
  # -y skips prompts; --foreground blocks until the delete actually finishes
  # (default is async, which would make the df reclaimed figure wrong).
  # No -f: without it, removal refuses on untracked files — a safeguard we
  # want. Ignored files (node_modules) do not block it. Our candidates are
  # dirty=0 (porcelain counts untracked), so they remove cleanly.
  "$WT" -C "$W" remove -y --foreground
else
  MAINDIR=$($GIT -C "$W" rev-parse --git-common-dir | xargs dirname)
  $GIT -C "$MAINDIR" worktree remove "$W"     # prunes metadata atomically
fi
```

- **Tier 1 — orphan metadata** (`git -C "$MAIN" worktree prune -v`). Zero risk; offer first.
- **Tier 2 — DELETE-MERGED.**
- **Tier 3 — DELETE-CLOSED** (show closure reason so the user can spot anything pending re-open).
- **Tier 4 — REVIEW** buckets: never auto-delete; list with the reason and let the user decide per item.

If removal fails ("contains modifications"), something changed since
the probe. Re-probe and move to REVIEW — do **not** reach for `--force`
unless the user names the path and accepts the loss.

---

## Phase 5: Verify & report

```bash
$GIT -C "$MAIN" worktree list | grep -c prunable   # expect 0
df -h "$WS" | tail -1                               # more free
```

Report: reclaimed (start → end free space), counts removed per bucket,
protected list, and the REVIEW queue left for the user.

---

## Common pitfalls (earned)

- **Directory ≠ branch.** Real and common — a worktree dir named for
  one ticket is checked out on an unrelated branch. Always look up the
  PR by `branch --show-current`.
- **Squash merges hide "merged".** Ancestry checks miss them; gate on
  `gh pr list`.
- **`gh` has no `-C`.** `cd` into the worktree instead.
- **No-remote repos.** A personal workspace root can be a local-only
  git repo; `gh` errors there. Exclude repos whose origin isn't GitHub.
- **Harness worktrees.** Editors/agents create their own worktrees
  (e.g. `.claude/worktrees/*`). Never touch them.
- **Shared stash.** `git stash list` in any worktree shows the parent
  repo's stashes — a high count is not per-worktree WIP.
- **Force-removal is not recovery.** A refused remove is the safeguard
  working; inspect and resolve deliberately.

## Refusals

Decline and ask for clarification if the user points the skill at `~/`
or `/`, or asks to skip the protect-list / per-tier confirmation, or
requests `--force` on a dirty worktree without naming the path and
accepting the loss.
