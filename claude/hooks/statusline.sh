#!/usr/bin/env bash
# Claude Code statusLine.
# Layout: cwd · branch · ctx% · model · effort · [style]
#
# Effort isn't in the statusLine JSON, so it's derived (in priority):
#   CLAUDE_CODE_EFFORT_LEVEL env → effortLevel in ~/.claude/settings.json
#   → model default (xhigh on Opus 4.7, high on Opus/Sonnet 4.6).

set -u

input=$(cat)
jqr() { printf '%s' "$input" | jq -r "$1 // empty" 2>/dev/null; }

cwd=$(jqr '.workspace.current_dir')
[ -z "$cwd" ] && cwd=$(jqr '.cwd')
model=$(jqr '.model.display_name')
model_id=$(jqr '.model.id')
pct=$(jqr '.context_window.used_percentage')
pct=${pct%%.*}
pct=${pct:-0}
style=$(jqr '.output_style.name')

D=$'\033[2m'; B=$'\033[1m'
C=$'\033[36m'; G=$'\033[32m'; Y=$'\033[33m'; R=$'\033[31m'; M=$'\033[35m'
RST=$'\033[0m'
SEP="${D} · ${RST}"

disp_cwd=${cwd/#$HOME/\~}

branch=$(git -C "$cwd" symbolic-ref --short HEAD 2>/dev/null \
      || git -C "$cwd" rev-parse --short HEAD 2>/dev/null)

if   (( pct >= 90 )); then ctx_c=$R
elif (( pct >= 70 )); then ctx_c=$Y
else                       ctx_c=$G
fi
ctx="${ctx_c}${pct}%${RST}"

effort="${CLAUDE_CODE_EFFORT_LEVEL:-}"
if [ -z "$effort" ] || [ "$effort" = "auto" ]; then
  effort=$(jq -r '.effortLevel // empty' "$HOME/.claude/settings.json" 2>/dev/null)
fi
if [ -z "$effort" ]; then
  case "$model_id" in
    *opus-4-7*)              effort="xhigh" ;;
    *opus-4-6*|*sonnet-4-6*) effort="high"  ;;
  esac
fi

line="${D}${disp_cwd}${RST}"
[ -n "$branch" ] && line+="${SEP}${G}${branch}${RST}"
line+="${SEP}${ctx}"
[ -n "$model" ]  && line+="${SEP}${B}${C}${model}${RST}"
[ -n "$effort" ] && line+="${SEP}${M}${effort}${RST}"

if [ -n "$style" ] && [ "$style" != "default" ]; then
  line+="${SEP}${D}${style}${RST}"
fi

printf '%s' "$line"
