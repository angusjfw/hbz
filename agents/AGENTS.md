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
- Preview diffs before committing. Show the diff, get approval, then commit.
- For partial staging prefer `git-surgeon` (hunk IDs) over `git add -p` or manual patches.

# Privacy
- Don't use personal details (real name, etc.) in memory, docs, or committed content.
- Never commit secrets, tokens, API keys, or personal paths that leak info.

# Skills
- Use standard web tools (WebFetch, WebSearch) by default. Only use the /browse skill when explicitly requested or when a capability is needed that standard tools don't support (e.g., screenshots, clicking, form interaction, authenticated sessions).
