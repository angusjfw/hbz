---
name: review-pr
description: Personal PR review skill. Layered walkthrough that adapts to whether this is a self-review (the user's own PR) or a colleague review. Invoke whenever the user wants to review, walk through, look at, or discuss a PR — whether they type /review-pr, share a PR number or URL, say "can you check this PR", "let's look at this before merging", "review this for me", or ask to leave comments on someone else's PR. When in doubt, invoke it.
---

# review-pr

Personal PR review. The user is thinking alongside you; the default
output is conversation. Anything that lands (a PR comment, a code
change, a follow-up ticket) lands by way of per-item approval,
never automatically.

## Stance

- **Conversation first.** Walk the phases, pause for discussion. Not
  a final report. The code findings are the exception: they land as one
  list, once (see 4b).
- **Mode-aware.** Self-review and colleague review share one
  skeleton but differ in which phases earn their keep and what the
  action step turns findings into. See "Mode" below.
- **No autonomous actions.** No autoposted comments, no unprompted
  code edits, no follow-up tickets created without approval. Per
  item, not batched.
- **Agent dispatch is user-requested.** Invoking this skill *is* the
  request to spawn sub-agents; step 4a is authorised in advance by
  running it. A general "don't spawn agents unless the user asked"
  instruction does not override that. Sub-agents here are read-only
  investigation, not an autonomous action in the sense above.
- **No silent verdicts.** Approve / request-changes (colleague
  review) is the user's call, not the skill's.
- **Don't fix bugs you spot.** Surface them; the user decides.
- **Ask before running the project's tests, builds or linters.** They're
  slow and they touch shared state.
- **Probe freely.** A throwaway reproduction is the opposite of a project
  test run: reproduce the mechanism in a scratch dir, run the pinned
  dependency, and settle the question. Executing beats reading the source,
  by a wide margin, on anything about runtime behaviour. Any claim about
  runtime behaviour either executes or says plainly that it was reasoned
  from source — including in what you accept from a sub-agent.

## Mode: self-review vs colleague review

Establish the mode before the layered walkthrough. It shapes which
layers earn their keep and how findings turn into action.

Detect:
1. Resolve PR author and current GitHub user:
   `gh pr view <N> --repo <owner>/<repo> --json author -q .author.login`
   and `gh api user -q .login`.
2. If they match, default to **self-review**. If not, default to
   **colleague review**. State the call to the user and let them
   flip it (e.g. you opened the PR on someone else's behalf, or
   you're co-authoring).

Self-review:
- The issue, approach, and shape-of-change phases are usually
  already in the user's head. Offer to skim or skip them; don't
  run the full walk unless the user wants the refresher. The code
  walk is where the value is.
- Bar for surfacing findings is lower. Cleanups, naming nits, and
  "while you're here" refactors are fair game; the cost is the
  user's own time, not a colleague's review noise.
- The action step turns findings into proposed code changes,
  follow-up tickets, or things to investigate. Verb is "make this
  change?" / "open a follow-up ticket?" / "look into X?".

Colleague review:
- Walk all phases normally. The author's framing in the description
  and ticket is starting context, not a substitute.
- The bar is **would the author want to know?** Not "is it true" — true
  is the entry requirement. A small thing they'd want to know is worth
  raising; a correct observation nobody would act on isn't, and doesn't
  go in the list at all. A noisy review gets dismissed wholesale, and a
  list of correct trivia is noise.
- The action step turns findings into draft PR comments. Verb is
  "draft a comment for this?".

## Worktree setup

The review runs on a local worktree because you'll need to run git
commands, read surrounding code (not just the diff), and use shell
tools throughout. Without it you're reviewing blind. The PR branch
and its base both need to be up to date with the remote before you
start, so diffs and context are accurate.

1. Resolve the PR's head and base branches:
   `gh pr view <N> --repo <owner>/<repo> --json headRefName,baseRefName`.
2. `wt switch -c <headRefName>` to create-or-switch into a worktree
   on the PR's branch (worktrunk creates if missing).
3. Sync the head branch. Behind, or diverged from a force-pushed
   remote, are both routine; resync a clean worktree without
   prompting.
   - `git fetch origin <headRefName>`.
   - If local is behind origin: `git pull --ff-only`.
   - If local has diverged but the worktree is clean and the only
     local-only commits are an earlier remote tip (typical
     force-push case): `git reset --hard origin/<headRefName>`.
4. `git fetch origin <baseRefName>` so diffs against
   `origin/<baseRefName>` reflect current base. No need to check
   out the base branch.
5. Verify:
   - `git rev-parse --abbrev-ref HEAD` matches `<headRefName>`.
   - The local branch's tip matches `origin/<headRefName>`.
   - `origin/<baseRefName>` has been updated by step 4.

Stop and surface to the user when judgement is needed:
- Uncommitted changes a sync would discard.
- Local commits that aren't on the remote (real unpushed work, not
  a force-push artifact).
- Branch mismatch after `wt switch`.
- Network or auth failure on fetch or pull.
- Verification still doesn't match after sync.

## Layered walkthrough

Walk these in order. **Pause and check in with the user after each
phase.** This is a discussion, not a final report. For self-review,
the first three phases are typically light or skipped; confirm with
the user up front rather than auto-skipping. For colleague review,
walk them normally.

### 1. Issue

What problem is this PR solving? Read the PR description in full.
Follow all linked Linear / GitHub tickets, including ones the PR claims
to close — they may not actually match. A PR claiming "Closes XYZ-123"
doesn't mean XYZ-123's scope is what got built.

Read the PR's existing comments — top-level review comments, inline
review comments, and the conversation timeline. Treat them like Linear
thread context: things already raised, decisions reached, points the
author has explained-away, and bot findings (Cursor, Bugbot, CodeRabbit)
the author has accepted, dismissed, or acknowledged.

**Establish where the PR sits in its arc, and what's deliberately
unfinished.** A ticket sequence, a stub handler waiting on the next
ticket, a flag left off, a schema field added ahead of its use, a
follow-up already filed — all of that is intended, not missing. Get it
explicitly and write it down, because everything downstream needs it: the
sub-agents read the diff cold and will otherwise hand back the whole arc's
remaining work as gaps, and reviewing a mid-arc step as if it were the
finished thing is the fastest way to waste an author's time.

Tools:
- `gh pr view <N> --repo <owner>/<repo> --json title,body,author,baseRefName,headRefName,additions,deletions,url`
- `gh pr view <N> --comments` for the conversation timeline
- `gh api repos/<owner>/<repo>/pulls/<N>/comments` for inline review comments
- `gh api repos/<owner>/<repo>/issues/<N>/comments` for top-level conversation comments
- `mcp__plugin_linear_linear__get_issue` for ticket detail
- `mcp__plugin_linear_linear__list_comments` for in-thread context
  (implementation plans buried in long comments are common)

Don't move on until the user/business need is clear. Inferred is not
enough.

### 2. Approach

From the description and the high-level diff (file list, +/- per file),
what's the shape of the change?

- Is this architecturally a sensible way to solve the issue?
- Are there obvious alternatives worth surfacing?
- Watch for fix-the-class-of-bugs vs minimum-diff judgment calls.
  Surface them rather than deciding silently.

Tools:
- `gh pr view <N> --json files` for per-file +/-
- `gh pr diff <N> --name-only` for file list
- `git diff <baseRefName>..HEAD --stat` from the worktree

### 3. What's changed (chunky)

Group the diff by intent: feature work vs refactor vs config vs tests
vs fixtures. Note anything surprising or out-of-scope.

For draft PRs: flag WIP markers (TODOs, debug prints, half-finished
tests) the user might want to clean up before marking ready.

**How to present:** Open with a one- or two-sentence narrative of what
the PR is actually doing ("the main change is X; the rest is supporting
config and tests"). Then show the intent groupings as structure
underneath that. The groupings exist to orient the code walk,
not as an end in themselves — say so. If you only print labelled buckets
without a framing sentence, the output is mechanical and loses the point.

Tools:
- `gh pr diff <N>` (save with `--patch` if large)
- `git diff <baseRefName>..HEAD` from the worktree

### 4. Code

The code *walk* comes after the phases above (or after the mode-driven
skip); the agent sweep in 4a launches earlier, in the background. The user
names what to dig into; don't go line-by-line through everything. Read
surrounding code, not just the diff, when context matters.

Four sub-steps: gather, filter, categorize, act.

#### 4a. Targeted review by aspect

**This is an agent dispatch step.** Use the Agent tool to spawn
sub-agents. They bring specialized perspective a read of the diff
doesn't, so the dispatch is the step, not a substitute for one.

**Dispatch as soon as you know the shape of the change** — during phase 2
or 3, not after. They run in the background while you and the user walk
the early phases, which is the difference between the sweep feeling free
and feeling slow. Say in one line that it's running; don't report on it
again until you have the list.

Spawn the sub-agents as a basic full sweep of the whole PR — every
changed file, not just the area the user named for the deeper dive.
This is broad baseline coverage; it backstops the targeted semantic
review you and the user do on the key changes, it doesn't replace it.
Pick the dimensions that apply to *this* PR (informed by the intent
grouping); don't blanket-dispatch every dimension, but each one you
run covers the full diff. Pick from:

- **General quality** (`code-reviewer`) — project rules in
  CLAUDE.md/AGENTS.md, framework conventions, naming, declarations,
  logging, perf, security. Bug detection.
- **Error handling** (`silent-failure-hunter`) — silent failures,
  broad catches, fallbacks that mask errors, missing logging, error
  message quality, error propagation.
- **Tests** (`pr-test-analyzer`) — behavioral coverage gaps, test
  quality, missing edge cases, flaky-prone patterns.
- **Comments** (`comment-analyzer`) — accuracy vs the code, rot,
  completeness for non-obvious bits. Not just comments the diff added —
  existing comments the change should have updated but didn't. On a
  prose-heavy diff, bound it explicitly: a claim that would mislead
  someone acting on it, not incompleteness, clarity or tone. Hundreds of
  new lines of docs will otherwise yield an unbounded supply of findings
  that are true and worth nothing.
- **Types** (`type-design-analyzer`) — encapsulation, invariants,
  usefulness of the abstraction.
- **Simplification** (`code-simplifier`) — duplication, complex
  bits that could be cleaner. Run only after the above pass; it's
  polish, not diagnosis.

Each agent is invoked via the Agent tool with `subagent_type` set
to its name. They live as personal agents at `~/.claude/agents/`
and are not plugin-namespaced. Run each over the full set of changed
files by default; only split an agent across file subsets for a PR
too large for one pass, and then so the splits cover all of it, never
to review only part.

**Coverage is a sweep property; the review list isn't.** Sweeping every
changed file is about not missing things, and a dimension that comes back
with nothing you'd raise did its job. Never let the size of the sweep
argue for the size of the list — four agents each handing back their best
three is not a twelve-item review, and the effort spent is not a reason to
spend the author's.

**Every dispatch brief carries, in the prompt:**
- Where the PR sits in its arc and what's deliberately unfinished
  (phase 1). Otherwise the agent reports the arc's remaining work as gaps.
- The bar: would the author want to know? Say it explicitly in colleague
  review — the agents are tuned for finding things, not for deciding
  what's worth someone's time, and this is the only place you can tell
  them before they've spent their effort.
- Findings must be about what **this diff does**. Pre-existing behaviour
  the diff merely sits next to is out. The exception is where the diff
  creates a new consumer that makes existing code newly wrong, and then
  the burden is showing the diff caused it.
- A finding needs a concrete fix the agent would actually make. "This is
  imprecise" with no alternative isn't a finding.
- Any claim about runtime behaviour executes or says it was reasoned from
  source.

Each diagnostic agent verifies its own findings internally: it dispatches
an independent `skeptic` sub-agent per candidate finding and returns only
those the skeptic scores ≥ 80, with a verdict and steelman attached. That
filtering is meant to be invisible to you — you receive verified findings,
not the verification (see 4b). (`code-simplifier` is exempt; it's polish,
not findings.)

**When the plumbing leaks, absorb it.** Skeptic verdicts sometimes arrive
at this level instead of at their parent agent — full Score / Verdict /
Steelman prose, one per candidate finding, arriving mid-conversation. That
is leakage, not review output. Never relay a verdict's prose to the user,
never present a finding *because* its verdict happened to land, and never
narrate the bookkeeping — which agent is quiet, whose report is missing,
which verdict contradicts which. Reconciling that is your job and the user
should not be able to tell it happened.

Dispatch all chosen agents in one message so they run concurrently, then
wait. Notification is the expected channel, so don't create tracking tasks
or schedule wakeups. But a notification is not a report: if an agent
finishes without sending findings, its report was lost somewhere and
waiting longer won't recover it — ask that agent directly (SendMessage)
for its findings and what it checked and found clean. Do that promptly and
silently rather than waiting, and don't wait on a dimension you can close
yourself by reading the code.

Size the sweep to the diff: a tiny change may need only one or two
agents, where the skeptic pass adds little; a large PR gets every
applicable dimension, split across file subsets that together cover all
of it. Agents run on Opus; drop to Sonnet only for mechanical drilling
(code search, caller/callee tracing).

Don't stop at the agents. They review the diff cold; you have the
worktree, the surrounding code, and the reason for the change. Add
your own findings from what they can't see — judge each change
against the callers, callees, and control flow around it, not the
hunk alone. Carry these into the synthesis and categorize steps
alongside the agent findings.

#### 4b. Cross-cutting synthesis

The per-finding adversarial filter now happens inside the agents (4a):
every finding you receive already survived an independent skeptic, with a
verdict and steelman attached. Don't re-litigate that finding by finding.
Your job here is the cross-cutting work no single agent could do — each
saw only its own dimension over the diff, not the whole picture:

1. **Dedup and merge across dimensions.** The same underlying issue can
   surface from several agents; merge them and keep all sources.
2. **Check against existing PR comments.** Has a finding already been
   raised by the author, another reviewer, or a bot? If so the bar to
   surface it again rises sharply — only re-raise if you genuinely
   disagree with how it was resolved, and frame it as engaging with the
   prior discussion, not a fresh finding.
3. **Mark what didn't go through a skeptic.** Anything you added from the
   worktree context (4a) has no skeptic verdict behind it. Carry it
   forward labelled as yours, so the categorize step can pitch its
   confidence honestly rather than borrowing the agents'.
4. **Cut, don't rank.** This is the step where the list gets short. Drop
   — not demote, drop — anything that is:
   - a form of "this isn't finished yet" on work the arc says isn't
     finished yet;
   - about behaviour the diff didn't introduce;
   - a complaint with no concrete alternative you'd actually make;
   - true and worth nothing. Correct is the entry requirement.

   In colleague review, one more, and it does the most work: **would the
   author want to know, or are you just showing you read it?** That's the
   test, not size — a leftover they forgot to delete, a comment someone
   will lean on while changing that code, an inconsistency with a sibling
   PR are all small and all worth raising. A correct observation nobody
   would act on is not, however solid.

   A dropped finding leaves no trace. It doesn't come back as a
   parenthetical or a "one more minor thing". The four criteria above
   apply in self-review too — they waste the user's time either way — but
   dropped items can legitimately resurface there as suggestions.

5. **A refuted finding is dead.** If a skeptic refuted it, it is gone —
   not present with a caveat, not moved to a lower bucket, not "my
   skeptic scored this low but". If you genuinely disagree with the
   refutation, say so as your own finding and carry your own reasoning;
   don't relay something you've been told is wrong and hedge it.

Carry survivors — the agents' verified findings plus your own confirmed
ones — into the categorize step, each with its source and the skeptic's
steelman so confidence can be expressed there.

**Present the list once.** No interim "here's what I have so far" cut, no
list that a still-running agent might amend. If you're waiting, say what
you're waiting on in one line and stop. When output arrives after you've
presented, amend the list in place — re-present the corrected list, or say
nothing if it doesn't change — never as a delta turn, a correction turn, or
a refinement to advice already given. A review delivered in five
instalments is unreadable however good each instalment is, and the user is
left assembling it.

#### 4c. Categorize and propose handling

The buckets differ by mode, because a bucket that exists will get filled.

**Colleague review — three headings:**

- **Blockers** — you'd hold the approval on these. Bugs, regressions,
  security, data loss, a contract the change gets wrong.
- **Nits but worth raising** — small, non-blocking, and the author would
  still want to know. A leftover, a comment that now misleads, an
  inconsistency with a sibling PR. Small is fine here; pointless isn't —
  everything in this bucket already passed the cut in 4b.
- **Strengths** — short, and only things genuinely worth saying.

Note what's missing: there's no take-it-or-leave-it tier. Anything that
would only have fitted there was dropped in 4b, and it doesn't come back
as a "minor" or a "while we're here" — that tier is where a reviewer's
credibility goes. The split between the first two buckets is about force,
not size: match the heading to whether you'd actually hold the approval on
it, so the author can tell a gate from a note.

**Self-review — four headings**, since the only cost is the user's time:

- **Critical** — bugs, regressions, security issues, data loss
  risks. Things that would break or harm users.
- **Important** — likely to cause problems but not certain.
  Missing test coverage on risky paths, error handling gaps,
  design concerns the author may not have weighed.
- **Suggestions** — quality, clarity, consistency. Take or leave.
- **Strengths** — anything notably well done, as a sanity-check that
  the dive wasn't one-sided.

Present findings as a plain numbered list under each heading. Each
item should read naturally: the agent that found it in brackets
(`[code-reviewer]`, `[pr-test-analyzer]`, etc.), then the finding,
then the file and line, then the fix you'd make. If you merged findings
from multiple agents, list all sources. No internal tracking labels (A,
T4, B/2/3, etc.) in the output.

Confidence lives in the placement and, where it genuinely helps, a brief
qualifier ("certain" / "plausible" / "worth checking") — never a raw
number. A qualifier is not a way to keep a finding you should have cut: if
it needs "this might be nothing, but", it was nothing.

Example format:
- `[silent-failure-hunter]` compliance.ts:20 — null FS bypasses
  gateway upper bound; description says gateway is enforced when
  set. Reachability is low, but the invariant is broken.
  *(considered: ASY-2374 retires this path, but that's not merged
  yet)*

Default proposal: in colleague review, blockers and nits together — the
list is short enough by now that splitting the action step adds nothing.
In self-review, Critical and Important now, with Suggestions and Strengths
offered separately so they don't crowd the action step.

#### 4d. Take action

Default: nothing happens without explicit, per-item approval.

**Ask once.** Put the choice to the user at the end of the findings list
and then hold. Don't re-offer to draft while you're already waiting on
them, and don't re-ask because something new arrived — new substance goes
into the list, not into another prompt. Per-item approval means the user
picks item by item once they've answered, not that you ask repeatedly.

**Self-review.** For each finding the user wants to act on, choose
with them per-item:
- Make the change here in the worktree (Edit tool). Show diff,
  don't auto-commit.
- Open a follow-up ticket (Linear). Draft title and body in
  conversation; only create after approval.
- Note as something to investigate later, no immediate action.

Phrase it as "make this change?" / "open a follow-up?" / "look
into this later?", not "draft a comment".

After applying a batch of in-worktree fixes, offer (don't force) a
targeted re-review: re-run the relevant dimension agent over just the
changed hunks to confirm the fix holds and didn't introduce a new issue.
Opt-in — skip it on trivial edits the user is confident in.

**Colleague review.** For each finding the user wants to land:
- Draft the comment in conversation. Show exact text and target
  (file path, line range, inline vs top-level review comment).
- User edits, approves, or rejects per comment.
- Post via the appropriate `gh pr review` invocation only after
  per-comment approval. One approval = one comment posted.

Phrase it as "draft a comment for this?", not "make the change".
Approve / request-changes verdicts are the user's call, not the
skill's. Never selected unprompted.

## Subagent drilling

For deeper investigation within a layer (trace callers, diff a
directory against base, summarise a long file's intent, look up
cross-repo references), spawn a subagent rather than burning the
conductor's context. Pass the layer's framing into the subagent prompt
so it stays scoped:

> "Reviewing PR #N in <repo> — assessing the approach. The change
> moves persistence from X to Y. Look at how X is used elsewhere in the
> codebase and report back: is this move clean, or are there other call
> sites that'll break? Under 200 words."

The conductor integrates subagent results into the layered conversation
without duplicating their findings.

## Variants

- **Approach-only.** Stop after the approach phase. Used when the
  user wants to consider the design without details.
- **Draft / not-ready.** Same flow, plus explicit WIP-marker flagging
  in the "what's changed" phase. No verdict — it isn't finished.
- **Targeted question.** User has a specific question (e.g. "does the
  widget reuse the existing component, and if not, why?"). Run the
  layers but answer the question as part of the layer where it fits,
  then stop.

These variants are responses to user signals during the conversation,
not a pre-flight survey. Walk normally; adapt when the user signals
which variant.

## Tooling reference

- `gh pr view <N> --repo <owner>/<repo>` — PR metadata
- `gh pr diff <N> --repo <owner>/<repo>` — full diff (`--patch` to save)
- `gh pr checkout <N>` — fallback if `wt switch -c` is unavailable
- `gh pr review <N> --comment --body <text>` — top-level review comment
  (action step only, with explicit approval)
- `gh api repos/<owner>/<repo>/pulls/<N>/comments` — inline review
  comments at file/line (action step only, with explicit approval)
- `gh pr view <N> --json author -q .author.login` — PR author (mode detection)
- `gh api user -q .login` — current GitHub user (mode detection)
- `mcp__plugin_linear_linear__get_issue` — Linear ticket detail
- `mcp__plugin_linear_linear__list_comments` — Linear thread comments
- `mcp__plugin_linear_linear__save_issue` — create follow-up ticket
  (action step, self-review only, with explicit approval)
- `git diff <base>..HEAD` and friends — once in the worktree
