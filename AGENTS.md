# hbz

Dotfiles for a minimal, keyboard-driven dev setup. Public repo.

## Conventions
- Makefile + symlinks for installation. Each tool gets a directory and a make target.
- Conditional sourcing for platform differences (`uname` checks, overlay files like `.zworkprofile`).
- Test Makefile targets locally before committing.
- Writing style: terse, clean. Selective emoji for section headers only.

## What's here
- `zsh/`, `vim/`, `tmux/`, `ghostty/` — core config (shared across platforms)
- `agents/` — global AI instructions (symlinked to `~/.claude/CLAUDE.md`)
- `claude/` — Claude Code settings and hooks (symlinked to `~/.claude/`)
- Platform-specific stuff lives in overlays or conditional blocks, not separate copies.

## Secrets
This is a public repo. Never commit secrets, tokens, API keys, or personal paths that leak info.
AI tool config that contains secrets must be gitignored.
