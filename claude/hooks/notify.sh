#!/bin/bash
# Notification hook — macOS notification + tmux display-message
# Skips if user is already focused on the triggering pane

read -r input

message="${input#*\"message\":\"}"
message="${message%%\"*}"
[ -z "$message" ] && message="Claude Code"

branch=$(git -C "$PWD" branch --show-current 2>/dev/null)
title="${branch:+[$branch] }${1:-Terminal}"

if [ -n "$TMUX" ]; then
    # Find which pane triggered this hook
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
    active_pane=$(tmux display-message -p '#{pane_id}')

    if [ "$pane_id" = "$active_pane" ]; then
        # Same pane — only notify if terminal isn't focused
        frontmost=$(osascript -e 'tell application "System Events" to get name of first application process whose frontmost is true' 2>/dev/null)
        if [ "$frontmost" != "ghostty" ]; then
            osascript -e "display notification \"$message\" with title \"$title\" sound name \"Ping\"" 2>/dev/null &
        fi
    else
        # Different pane — fire both
        osascript -e "display notification \"$message\" with title \"$title\" sound name \"Ping\"" 2>/dev/null &
        if [ -n "$pane_id" ]; then
            window_info=$(tmux display-message -t "$pane_id" -p '#{window_index}:#{window_name}')
            tmux display-message "[$window_info] $message"
        fi
    fi
else
    # No tmux — just macOS notification
    osascript -e "display notification \"$message\" with title \"$title\" sound name \"Ping\"" 2>/dev/null &
fi
