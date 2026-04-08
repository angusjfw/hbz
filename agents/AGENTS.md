# Rules
- Verify before asserting. Never state file contents, system state, or outcomes without checking first. Read the file, run the command, then speak.
- Stay in scope. Don't make changes beyond what was explicitly agreed. Ask questions, raise issues, discuss tradeoffs and get approval before acting.
- Update specs, todos and plans before acting on them. Keep external context (docs, design specs) current as decisions are made, not after the fact.
- Don't use personal details (real name, etc.) in memory, docs, or committed content.
- Never commit secrets, tokens, API keys, or personal paths that leak info.
- Don't print commands for the user to run. Run them directly, or in a tmux pane if interaction is needed. If unsure whether to proceed, ask first.
- Work autonomously until user input is genuinely needed. Check settings to know whether an action requires approval (don't infer). At those checkpoints, offer interactive options (open diff in a pane, run in a pane, open in nvim).

# Writing style
- Terse, clean. No AI voice; no emdash chains, no filler, no "let's dive in".
- Avoid terms that carry domain baggage outside their domain ("prior art", "blast radius"). Write plainly for the context at hand.

# Git
- Always start new work in a worktree branch. Use `wt` (worktrunk). Consult the worktrunk skill.
- Surgical commits. Each one coherent and meaningful. One logical change per commit.
- Use amends, fixup commits and rebases to keep history legible for reviewers. Update commit messages as necessary. Autosquash fixups directly; open a tmux pane for rebases that need user input.
- Conventional commit prefixes: `feat`, `fix`, `docs`, `refactor`, `chore`, etc.
- Imperative mood, max 72 chars summary, body explains what and why.
- Review the diff before committing or pushing. If something looks off, fix it first.
- For partial staging prefer `git-surgeon` (hunk IDs) over `git add -p` or manual patches.
- Always create PRs as drafts unless explicitly told to open them as ready for review.

# tmux
- Use tmux panes for showing output and deeper interactivity when the user's input is needed or requested.
- Be aware of the tmux layout and external changes to it when creating, sending to or killing panes (`tmux list-panes -a -F '#{window_index} #{pane_id} #{pane_current_command} #{pane_current_path}'`).
- Consider good arrangement, sizing and reuse of the panes. Prefer vertical split for tools alongside conversation, horizontal for output-heavy content.
- Claude's own pane is `$TMUX_PANE`. Print new pane IDs (`-P -F '#{pane_id}'`)
    to reference internally.
- Open files for editing/review in nvim. If a vim pane exists, you may open a new tab in it (`tmux send-keys -t {pane_id} Escape ':tabnew +{line} {file}' Enter`); but be aware of trampling the user's session.
- Open diffs in a tmux pane when reviewing with the user.
- Git output in panes: `bash -c 'git <cmd> --color=always | less -R'` (flags before paths). Don't pipe git output through vim.
- Run servers, test suites, and other user-facing output in panes, not inline.
- Read pane output with `tmux capture-pane -p -t {pane_id} -S -50` rather than re-running commands.
- Untracked files: `git diff` shows nothing for new files, use `git add --intent-to-add` first.

# Screenshots and visual verification
- Use screenshots to verify visual changes yourself rather than asking the user. Reach for this whenever the result is visual.
- Capture frontmost window: `screencapture -x /tmp/screenshot.png`.
- Capture another app: use osascript to hold focus: `osascript -e 'tell application "App" to activate' -e 'delay 3' -e 'do shell script "screencapture -x /tmp/shot.png"'`
- After capturing, read the image. If something's wrong, fix it. If you need user input on what you're seeing, open it for them.

# Web search / browsing
- Use standard web tools (WebFetch, WebSearch) by default. Only use the /browse skill when explicitly requested or when a capability is needed that standard tools don't support (e.g., screenshots, clicking, form interaction, authenticated sessions).
