# hbz

Dotfiles for a minimal, keyboard-driven dev setup. Public repo.

## Conventions
- Makefile + symlinks for installation. Each tool gets a directory and a make target.
- Conditional sourcing for platform differences (`uname` checks, overlay files like `.zworkprofile`).
- Writing style: terse, clean. Selective emoji for section headers only. No AI voice.
- Don't rewrite existing prose or config comments unless asked.
- Don't add features, refactor, or "improve" beyond what's discussed.
- Surgical commits. Each one coherent and meaningful. No AI attribution.
- Conventional commit prefixes: `feat`, `fix`, `docs`, `refactor`, `chore`, etc.
- Imperative mood, max 72 chars summary, body explains what and why.

## What's here
- `zsh/`, `vim/`, `tmux/` — core shell/editor/multiplexer config (shared across platforms)
- `docs/design.md` — working spec for the current overhaul
- Platform-specific stuff lives in overlays or conditional blocks, not separate copies.

## Secrets
This is a public repo. Never commit secrets, tokens, API keys, or personal paths that leak info.
AI tool config that contains secrets must be gitignored.
