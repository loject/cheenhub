---
name: commit
description: Review current git changes and create meaningful git commits. Use when the user explicitly asks to commit changes, types `/commit`, asks to "закомить", "сделай коммит", "commit everything", or wants unrelated changes split into separate commits with clear messages.
---

# Commit

Inspect the current git worktree before committing anything.

## Workflow

1. Run `git status --short` and inspect the diff that is about to be committed.
2. Check whether there are any staged changes.
3. If there are no staged changes, treat the full current worktree as the commit candidate and stage the relevant changes yourself before committing.
4. By default, interpret a generic commit request as a request to commit all current repository changes unless the user explicitly narrows the scope.
5. Group changes by logical feature or fix. If the worktree contains unrelated changes, create multiple commits instead of one mixed commit.
6. Stage only one logical group at a time.
7. Write a concise Conventional Commits message using the format `type: summary`.
8. Repeat until all intended changes are committed.
9. Report the created commit hashes and mention any files intentionally left uncommitted.

## Commit Message Format

Use Conventional Commits-style prefixes. The default format is:

```text
type: short imperative summary
```

Preferred types:

- `feat:` for user-facing features or product behavior.
- `fix:` for bug fixes and regressions.
- `docs:` for documentation-only changes.
- `test:` for tests-only changes.
- `refactor:` for behavior-preserving code structure changes.
- `perf:` for performance improvements.
- `style:` for formatting-only changes.
- `build:` for build system, dependency, or packaging changes.
- `ci:` for CI/CD configuration.
- `chore:` for maintenance that does not fit the above types.
- `revert:` for reverting a previous commit.

Examples:

- `feat: add server member settings`
- `fix: refresh closed realtime streams`
- `docs: update project todo list`
- `chore: warn on oversized rust files`

## Rules

- Do not amend existing commits unless the user explicitly asks.
- Do not push, merge, rebase, or sync branches as part of this skill.
- Do not use destructive git commands.
- If the user did not specify scope, the default scope is the full current repository worktree; do not silently exclude files unless the user explicitly narrows scope or asks to leave something out.
- If there are clearly unrelated user changes, preserve them and separate them into their own commit instead of folding them into another change.
- If commit grouping is ambiguous but still reasonably inferable from the diff, make the grouping decision and state it briefly.
- If a file looks generated or accidental, verify from the diff before committing it.
- Prefer the most specific Conventional Commits type that matches the staged change. Use `chore:` only when no more specific type fits.
