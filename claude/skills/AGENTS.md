# Skills

Rules for authoring and maintaining skills in this repo.

## Composition and de-coupling
- Decouple via discovery, not reference. A skill's body and description should not name other skills. Claude sees all available skills in the environment and picks them up as needed; cross-references create brittle dependencies that drift.
- Namespace coupled skills with a shared prefix (e.g. `claude-manager-*`) when they're operationally related. The prefix signals coupling at the naming level without requiring cross-references.

## Invocation
- Description drives invocation. The frontmatter description handles both slash command and natural-language matching. Body content guides behavior after invocation. Don't put invocation routing in the body.

## Coherence and scope
- Lean first. Add structure (sections, helper scripts, separate skills) only when there's a clear payoff. Don't pre-build for hypothetical needs.
- Skill bodies are operational; rulebooks own cross-cutting rationale. Keep the body focused on what to do; let AGENTS.md (or equivalent) carry the cross-cutting constraints (writing style, git practice, ceremony).
- Skill-generic vs project-specific. Skills should not encode project paths or conventions (journal location, wiki structure, default panes per task, worktree commands). Read those at runtime from the project's CLAUDE.md/AGENTS.md so the skill stays portable across personal and work setups.
- Vendor selectively. When borrowing from upstream skills, take only what earns its keep in this context. Honor the license (NOTICE, modifications log) when required.
