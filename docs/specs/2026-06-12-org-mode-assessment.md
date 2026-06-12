# Org-mode task/agenda layer: assessment (markdown instead)

Proposal: adopt org-mode (via nvim-orgmode) as the task/agenda layer —
TODO states, dates, agenda views — integrated with the session
workflow.

Decision: **no org**. The agenda capability is wanted; the format and
plugin are not. A thin markdown equivalent folds into the amx skill
layer instead.

## Findings (June 2026)

- nvim-orgmode is the one mature org implementation outside Emacs
  (active, monthly-ish releases) and genuinely covers the wanted
  layer: TODO states, SCHEDULED/DEADLINE with repeaters, agenda views,
  capture, refile, org-roam port. Alternatives are dead or unsuited
  (vim-dotoo archived, neorg paused on its own format, obsidian.nvim
  has no agenda engine).
- Costs for this setup: requires Neovim 0.11+ and a Lua/treesitter
  config (current config is shared vimscript .vimrc + vim-plug);
  introduces a second markup dialect into an all-markdown system
  (registry, journal, specs, stub log) that Claude handles natively in
  markdown; mobile/interop story is the documented practitioner
  bail-out point.
- The org features that justify a format switch (babel, export,
  column view) are exactly what nvim-orgmode lacks — the format cost
  would buy the agenda layer alone, and the agenda layer is buyable
  in markdown.

## Direction

Markdown agenda, thin, inside the amx effort:

- A `tasks.md` convention: items with states and optional dates, plain
  markdown, location decided during amx implementation.
- Agenda generation as judgment work in the amx skill layer (likely
  `amx-board`): today's view, capture, reschedule on demand, drawing
  on the registry and stub log for session-linked work.
- No new tools or formats. If the convention proves too thin in
  practice, evaluate Backlog.md (markdown task files + CLI/kanban,
  Claude Code integration) before anything heavier; reconsider
  nvim-orgmode only if a Lua config migration happens for independent
  reasons.
