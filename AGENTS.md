# hbz

Dotfiles for a minimal, keyboard-driven dev setup. Public repo.

## Conventions
- Makefile + symlinks for installation. Each tool gets a directory and a make target.
- Monorepo: subprojects with their own builds (e.g. Rust crates) live in their tool's directory, built via make targets, artifacts gitignored, binaries installed to `~/.local/bin`. No submodules or split repos.
- Conditional sourcing for platform differences (`uname` checks, overlay files like `.zworkprofile`).
- Test Makefile targets locally before committing.
- Commit changes directly to `main` — this repo doesn't use worktrees, feature branches or PRs.
- This is a config repo — changes to dotfiles, themes, and agent instructions are features/fixes, not docs/chores.
- Config managed here is symlinked into place. Edit the source files in this repo, not the symlink targets (e.g. edit `agents/AGENTS.md`, not `~/.claude/CLAUDE.md`).
- Writing style: terse, clean. Selective emoji for section headers only.
- Prefer vendor-neutral references where the tool is interchangeable (e.g. "terminal emulator" not a specific one). Core tools like tmux, vim, zsh are fine to name directly.

## What's here
- `zsh/`, `vim/`, `tmux/` — core config (shared across platforms)
- `ghostty/` — terminal emulator (macOS)
- `agents/` — global AI agent instructions, symlinked as `~/.claude/CLAUDE.md`
- `claude/` — Claude Code settings and hooks (symlinked to `~/.claude/`)
- `worktrunk/` — git worktree management (symlinked to `~/.config/worktrunk/`)
- `keyboard/` — ZSA Voyager layout: QMK source and firmware (repo is source of truth; flashed, not symlinked) + session-leds tooling
- `brew/` — Brewfile (macOS), `pacman/` — package lists (Arch)
- `arch/` — Arch/sway install notes, `screenshots/` — repo screenshots
- Platform-specific stuff lives in overlays or conditional blocks, not separate copies.

## Untracked files
- `docs/design.md` is gitignored — `git diff` won't show changes. The edit tool's write diff is usually sufficient; otherwise summarise from memory.

## Secrets
This is a public repo. Never commit secrets, tokens, API keys, or personal paths that leak info.
AI tool config that contains secrets must be gitignored.
