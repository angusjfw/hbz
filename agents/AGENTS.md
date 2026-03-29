# Rules
- Verify before asserting. Never state file contents, system state, or outcomes without checking first. Read the file, run the command, then speak.
- Stay in scope. Don't make changes beyond what was explicitly agreed. Ask questions, raise issues, discuss tradeoffs and get approval before acting.
- Update specs, todos and plans before acting on them. Keep external context (docs, design specs) current as decisions are made, not after the fact.

# Writing style
- Terse, clean. No AI voice — no emdash chains, no filler, no "let's dive in".

# Commits
- Surgical commits. Each one coherent and meaningful. One logical change per commit.
- Use amends, fixup commits and rebases to keep history legible for reviewers. Update commit messages as necessary.
- Conventional commit prefixes: `feat`, `fix`, `docs`, `refactor`, `chore`, etc.
- Imperative mood, max 72 chars summary, body explains what and why.
- No AI attribution or co-author lines.
- Preview diffs before committing. When in tmux, open diffs in a split pane. Get approval, then commit.
- For partial staging prefer `git-surgeon` (hunk IDs) over `git add -p` or manual patches.

# Terminal tools (tmux)
- When in tmux, prefer opening tools in panes over showing output inline.
- Git output in panes must use: `bash -c 'git <cmd> --color=always | less -R'` (keeps pane open with vim-style navigation).
- Untracked files: `git diff` shows nothing for new files. Use `git add --intent-to-add <paths>` first, then `git diff`.
- After opening a pane, verify it has content with `tmux capture-pane -p -t {pane_id} -S -3`.
- Vertical split (`-h`) for tools alongside conversation (diffs, file review). Horizontal split (`-v`) for output-heavy content (test runners).
- Open nvim for file editing/review: `tmux split-window -h -c "#{pane_current_path}" "nvim +{line} {file}"`
- To read pane output without re-running: `tmux capture-pane -p -t {pane_id} -S -50`

# Screenshots and visual verification
- Use screenshots to verify visual changes yourself rather than asking the user. Reach for this whenever the result is visual.
- Capture frontmost window: `screencapture -x /tmp/screenshot.png`.
- Capture another app: use osascript to hold focus: `osascript -e 'tell application "App" to activate' -e 'delay 3' -e 'do shell script "screencapture -x /tmp/shot.png"'`
- After capturing, read the image. If something's wrong, fix it. If you need user input on what you're seeing, open it for them.

# Privacy
- Don't use personal details (real name, etc.) in memory, docs, or committed content.
- Never commit secrets, tokens, API keys, or personal paths that leak info.

# Skills
- Use standard web tools (WebFetch, WebSearch) by default. Only use the /browse skill when explicitly requested or when a capability is needed that standard tools don't support (e.g., screenshots, clicking, form interaction, authenticated sessions).
