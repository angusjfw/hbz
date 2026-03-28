#!/bin/bash
# Notify when Claude Code needs attention

read -r input

message="${input#*\"message\":\"}"
message="${message%%\"*}"
[ -z "$message" ] && message="Claude Code"

# 1. macOS notification — works regardless of terminal/focus state
osascript -e "display notification \"$message\" with title \"Claude Code\" sound name \"Ping\"" 2>/dev/null &

# 2. tmux status bar message — visible in-terminal regardless of which tab
if [ -n "$TMUX" ]; then
    # Walk up process tree to find which tmux pane owns this hook
    find_pane() {
        local pid=$$
        while [ "$pid" -gt 1 ] 2>/dev/null; do
            local match
            match=$(tmux list-panes -a -F '#{pane_id} #{pane_pid}' 2>/dev/null | while read -r id ppid; do
                if [ "$ppid" = "$pid" ]; then
                    echo "$id"
                    break
                fi
            done)
            if [ -n "$match" ]; then
                echo "$match"
                return
            fi
            pid=$(ps -o ppid= -p "$pid" 2>/dev/null | tr -d ' ')
        done
    }

    pane_id=$(find_pane)
    if [ -n "$pane_id" ]; then
        window_info=$(tmux display-message -t "$pane_id" -p '#{window_index}:#{window_name}')
        tmux display-message "[$window_info] $message"
    fi
fi
