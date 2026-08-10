#!/usr/bin/env bash
# tmux-resurrect save-command strategy: the pane's own foreground command.
#
# Symlinked into tmux-resurrect's save_command_strategies/ by `make tmux`
# and selected with `@resurrect-save-command-strategy 'pane-tty'`.
#
# Replaces the bundled `ps` strategy, which greps `ps -ao ppid,args` for
# lines starting with the pane pid. Two problems, both hit in practice:
#
#   - It emits every child, one per line. A Claude Code pane has two —
#     `claude` plus a sibling shell Claude spawns for its own tool calls
#     on a separate pty — so the extra line lands in the save file as a
#     stray record.
#   - `grep "^$PANE_PID"` is a prefix match, so pane pid 3378 also
#     matches an unrelated pid 33780.
#
# Both are fixed by matching ppid exactly and keeping only the child
# sharing the pane's tty: Claude's tool shell sits on a different one.

PANE_PID="$1"
[ -z "$PANE_PID" ] && exit 0

pane_tty="$(ps -p "$PANE_PID" -o tty= 2>/dev/null | tr -d '[:space:]')"
[ -z "$pane_tty" ] && exit 0

ps -ax -o pid=,ppid=,tty=,args= 2>/dev/null | awk -v pid="$PANE_PID" -v tty="$pane_tty" '
	$2 == pid && $3 == tty {
		$1 = ""; $2 = ""; $3 = ""
		sub(/^[ \t]+/, "")
		print
		exit
	}
'
