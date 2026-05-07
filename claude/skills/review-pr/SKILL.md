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
  a final report.
- **Mode-aware.** Self-review and colleague review share one
  skeleton but differ in which phases earn their keep and what the
  action step turns findings into. See "Mode" below.
- **No autonomous actions.** No autoposted comments, no unprompted
  code edits, no follow-up tickets created without approval. Per
  item, not batched.
- **No silent verdicts.** Approve / request-changes (colleague
  review) is the user's call, not the skill's.
- **Don't fix bugs you spot.** Surface them; the user decides.
- **Don't run tests, builds, linters** without asking. If verification
  matters, ask first.

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
- Bar for surfacing findings is higher (see the adversarial filter
  step). A noisy review gets dismissed wholesale.
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

Only after the phases above (or after the mode-driven skip). The user
names what to dig into; don't go line-by-line through everything.
Read surrounding code, not just the diff, when context matters.

Four sub-steps: gather, filter, categorize, act.

#### 4a. Targeted review by aspect

**This is an actual agent dispatch step.** Use the Agent tool to
spawn sub-agents. Do not substitute manual code reading for this step,
even if you have already read the diff. The sub-agents bring
specialized perspective you haven't applied. If you find yourself
thinking "I've already done this" — you haven't.

For the named area(s), spawn focused sub-agents on the dimensions
that apply to *this* PR (informed by the intent grouping from the
previous phase).
Don't blanket-dispatch every dimension. Pick from:

- **General quality** (`code-reviewer`) — project rules in
  CLAUDE.md/AGENTS.md, framework conventions, naming, declarations,
  logging, perf, security. Bug detection.
- **Error handling** (`silent-failure-hunter`) — silent failures,
  broad catches, fallbacks that mask errors, missing logging, error
  message quality, error propagation.
- **Tests** (`pr-test-analyzer`) — behavioral coverage gaps, test
  quality, missing edge cases, flaky-prone patterns.
- **Comments** (`comment-analyzer`) — accuracy vs the code, rot,
  completeness for non-obvious bits.
- **Types** (`type-design-analyzer`) — encapsulation, invariants,
  usefulness of the abstraction.
- **Simplification** (`code-simplifier`) — duplication, complex
  bits that could be cleaner. Run only after the above pass; it's
  polish, not diagnosis.

Each agent is invoked via the Agent tool with `subagent_type` set
to its name. They live as personal agents at `~/.claude/agents/`
and are not plugin-namespaced. Run each scoped to a specific set
of files with a focused prompt; score each issue by confidence
(0-100); only carry forward findings ≥ 80. Confidence scores are
for internal filtering — how to present them to the user is
handled in the categorize step.

#### 4b. Adversarial challenge

Most unfiltered agent findings turn out to be unnecessary, wrong, or
context-blind. Apply the filter here, before the user sees them, not
as pushback after the fact. The user has been explicit that this
filter needs to be sharp, especially in colleague review where the
cost of noise is high.

For each finding from 4a:

1. **State it.** Location, claim, suggested change.
2. **Steelman the existing code.** Why might it be deliberately
   like this? Convention in the file or module? Decision recorded
   in commit messages, ticket comments, or nearby tests? A
   constraint the agent didn't see (perf, ordering, framework
   gotcha, intentional duplication for clarity)?
3. **Cross-check against the wider diff and surrounding code.**
   The agent only saw what was passed in. Read the rest of the
   change and any callers or callees that bear on the claim.
   Look for evidence that contradicts the finding before evidence
   that confirms it.
4. **Check against existing PR comments.** Has this already been
   raised by the author, another reviewer, or a bot? If yes, the
   bar to surface it again rises sharply. Only re-raise if you
   genuinely disagree with how it was resolved, and frame it as
   engaging with the prior discussion, not as a fresh finding.
5. **Re-score confidence.** Drop anything below 80. Merge findings
   that overlap. Internal tracking labels (e.g. "A", "T4",
   "B/2/3") and raw numeric scores are for your own bookkeeping —
   do not carry them into user-facing output. For survivors,
   record which agent surfaced it and your post-steelman
   confidence, to be expressed in the categorize step below.

Bias toward dropping. A missed finding is a future discussion; a
noisy review gets dismissed wholesale. In colleague review, raise
the bar further: anything you'd be embarrassed to put in front of
the author should not survive this step.

#### 4c. Categorize and propose handling

Group survivors into:

- **Critical** — bugs, regressions, security issues, data loss
  risks. Things that would break or harm users.
- **Important** — likely to cause problems but not certain.
  Missing test coverage on risky paths, error handling gaps,
  design concerns the author may not have weighed.
- **Suggestions** — quality, clarity, consistency. Take or
  leave. In self-review, this category can be larger; in
  colleague review, only the highest-value ones cross the bar.
- **Strengths** — anything notably well done. Sanity-checks
  that the dive wasn't one-sided in self-review; worth saying
  to a colleague.

Present findings as a plain numbered list under each heading. Each
item should read naturally: the agent that found it in brackets
(`[code-reviewer]`, `[pr-test-analyzer]`, etc.), then the finding,
then the file and line. If you merged findings from multiple agents,
list all sources. Confidence lives in the category placement and
optionally a brief qualifier ("certain" / "plausible" / "worth
checking") — not as a raw number. No internal tracking labels (A,
T4, B/2/3, etc.) in the output.

Example format:
- `[silent-failure-hunter]` compliance.ts:20 — null FS bypasses
  gateway upper bound; description says gateway is enforced when
  set. Reachability is low, but the invariant is broken.
  *(considered: ASY-2374 retires this path, but that's not merged
  yet)*

Default proposal: handle Critical and Important items now; offer
Suggestions and Strengths separately so they don't crowd the action
step.

#### 4d. Take action

Default: nothing happens without explicit, per-item approval.

**Self-review.** For each finding the user wants to act on, choose
with them per-item:
- Make the change here in the worktree (Edit tool). Show diff,
  don't auto-commit.
- Open a follow-up ticket (Linear). Draft title and body in
  conversation; only create after approval.
- Note as something to investigate later, no immediate action.

Phrase it as "make this change?" / "open a follow-up?" / "look
into this later?", not "draft a comment".

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
