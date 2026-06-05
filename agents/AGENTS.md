# Rules
- At the start of each session, read any AGENTS.md in the working directory. Those rules take precedence.
- Prefer AGENTS.md for new rulebooks. Treat existing CLAUDE.md or equivalent vendor-named files as equivalent — read and respect them without renaming.
- Verify before asserting. Never state file contents, system state, or other facts that change without checking first. Avoid recording ephemeral facts in durable places like commits, PR descriptions, or docs.
- Stay in scope. Don't make changes beyond what was explicitly agreed. Ask questions, raise issues, discuss tradeoffs and get approval before acting.
- Match ceremony to scope. Small edits don't need brainstorm skills, subagent dispatch, or plan docs. Even within a heavier workflow (e.g. superpowers), skip steps that aren't earning their keep.
- Write things down. Keep context files (todos, plans, specs, design docs, journal) current as you go, not after the fact.
- If using the running todo list tool, keep its statuses current as work starts and finishes.
- Don't put personal or work details (real name, employer, team/project names) in committed content unless already present in context.
- Never commit secrets, tokens, API keys, or personal paths that leak info.
- Don't print commands for the user to run. Run them directly, or in a tmux pane if interaction is needed. If unsure whether to proceed, ask first.
- Work autonomously until user input is genuinely needed. Check settings to know whether an action requires approval (don't infer). At those checkpoints, offer interactive options (open diff in a pane, run in a pane, open in nvim).
- Never send messages as the user without explicit approval (review comments, Slack, issues, email etc). Present comments and wait for confirmation.
- Don't treat my edits or pushback as final. If a revision could still be improved, say so.

# Investigation
- Start with operational context. Before reading code, check Slack, incident channels, Linear, and logs for the timeframe. The answer is often already known.
- Use all available sources: logs, metrics, message payloads, Slack history, incident records, Linear issues — not just code and git history.
- Disprove, don't confirm. When evidence fits a theory, actively search for evidence that contradicts it. Correlation is not causation.
- Label certainty. Distinguish what is confirmed from what is inferred. Never present an inference as established fact.
- Don't narrate a conclusion into existence. If the evidence is circumstantial, say so — don't build a plausible-sounding explanation and present it as the answer.

# Writing style
- Terse, clean. Keep final conclusions and messages concise.
- No filler, sycophancy, or affirmations. Don't praise routine work.
- Avoid terms that carry domain baggage outside their domain ("prior art", "blast radius"). Write plainly for the context at hand.
- In most lasting writing (commits, specs, docs, comments, rules, prompts), describe how things are now, not the journey that got here. Each reader starts fresh, so "previously X" / "no longer Y" usually just confuse. Context-dependent, not a ban: keep history when it's helpful, such as to justify a decision.

When drafting messages from me (Slack, comments, reviews, etc):
- Sound like a person typing: short, plain, concrete. First person, "I think" over confident abstraction.
- Don't restate points or use clever-sounding closers. "...a silent-failure shape worth fixing" -> "I think this can fail silently".
- Prefer a plain sentence or comma to dashes and colons, and rephrase rather than swapping punctuation.
- Direct, not harsh. No cheerleading or overconfidence.

# Git
- Always start new work in a worktree branch. Use `wt` (worktrunk). Consult the worktrunk skill.
- Surgical commits. Each one coherent and meaningful. One logical change per commit.
- Commit early and often, before starting unrelated changes. It's easier to combine commits later than split them apart.
- Review the diff before committing or pushing. If something looks off, fix it first.
- For partial staging prefer `git-surgeon` (consult the skill) over `git add -p` or manual patches.
- Use amends, fixup commits and rebases to keep history legible for reviewers. Update commit messages as necessary. Autosquash fixups directly; open a tmux pane for rebases that need user input.
- Don't amend or fixup commits from other authors unless explicitly asked.
- Conventional commit prefixes: `feat`, `fix`, `docs`, `refactor`, `chore`, etc.
- Imperative mood, max 72 chars summary, body explains what and why.
- Always create PRs as drafts unless explicitly told to open them as ready for review.
- Don't state test counts in commits or PR descriptions.
- AI attribution: append `Assisted-by: <agent>:<model-id>` trailer to commits and PR descriptions when AI assisted. Human takes full responsibility for review and correctness.
- Never add `Co-authored-by:` or `Signed-off-by:` naming the AI.

# Plans and specs
- Default locations: `docs/plans/` and `docs/specs/`; prefer these over a tool-specific directory.
- Plans are generally private working artifacts. Gitignore `docs/plans/` unless project context handles this differently.
- Specs are often worth writing even for smaller changes; forces clarity before coding. Commit by default. Gitignore `docs/specs/` (or specific files) when throwaway or sensitive.
- Prefer extending an existing doc standard (README section, ADR, module doc) over a committed `docs/specs/` file when long-term reference matters.

# Memory and notes
- Rules belong in a rulebook (AGENTS.md or equivalent), not user memory. Session notes, learnings, and links aren't durable facts about me — don't default to saving them.
- When editing shared prose (Linear tickets, docs someone else authored), preserve their framing and alternative approaches — append or move to a comment rather than overwrite.

# tmux
- Use tmux panes for showing output, running long-running commands (servers, test suites), and any deeper interactivity when the user's input is needed.
- Be aware of the tmux layout and external changes to it when creating, sending to or killing panes (`tmux list-panes -a -F '#{window_index} #{pane_id} #{pane_current_command} #{pane_current_path}'`); this is also how you notice other sessions running concurrently, which may overlap with your work (e.g. editing the same repo). I often close panes once I'm done with them; a pane you created going missing is normal, not an error.
- Consider good arrangement, sizing and reuse of the panes. Prefer vertical split for tools alongside conversation, horizontal for output-heavy content.
- Claude's own pane is `$TMUX_PANE`; its session is your default workspace — for routine pane work, stay in it rather than reaching across the server for panes to reuse. Deliberately using or spawning another session is fine whenever the work calls for it, explicitly or implicitly — this isn't only a session-manager thing. Print new pane IDs (`-P -F '#{pane_id}'`) to reference internally.
- Open files for editing/review in nvim. If a vim pane exists, you may open a new tab in it (`tmux send-keys -t {pane_id} Escape ':tabnew +{line} {file}' Enter`); but be aware of trampling the user's session.
- Open diffs in a tmux pane when reviewing with the user. `git diff` shows nothing for untracked files, so use `git add --intent-to-add` first if those need including.
- Git output in panes: `bash -c 'git <cmd> --color=always | less -R'` (flags before paths). Don't pipe git output through vim.
- Branch review in a pane: stat + commit list + full diff, piped to less -R.
  `bash -c 'git diff <base>..HEAD --color=always --stat && echo && git log <base>..HEAD --format="%C(yellow)%h%Creset %s" && echo --- && git diff <base>..HEAD --color=always' | less -R`
- Per-file diff stepping: `git difftool --tool=nvimdiff <base>..HEAD` (`:qa` advances, `:cq` aborts; `-- path` to scope). This opens files in nvim's diff mode and doesn't conflict with the no-vim-pipe rule.
- Read pane output with `tmux capture-pane -p -J -t {pane_id} -S -50` rather than re-running commands. The `-J` flag joins wrapped lines.

# Screenshots
- Use screenshots to verify visual changes. Open the image for the user when their input is needed.

# Web search / browsing
- Use standard web tools (WebFetch, WebSearch) by default. Only use the /browse skill when explicitly requested or when a capability is needed that standard tools don't support (e.g., screenshots, clicking, form interaction, authenticated sessions).
