# Planning Guidelines For Agents

These guidelines apply to planning documents under `docs/planning`.

## Status Folders

- `to-do`: accepted future work that is not active yet.
- `in-progress`: active work that has enough shape to implement or refine.
- `complete`: finished work with a short completion summary.
- `canceled`: historical plans that should not drive new implementation unless the user explicitly revives them.

## Working With Plans

- Treat the active root crate as the source of truth; do not infer current architecture from canceled documents.
- Move a plan to `complete` only after its applicable tracked-repo work is finished or explicitly resolved.
- When completing a plan, update the document with what changed, what was verified, and any candidate work that no longer applies.
- Keep planning docs concise and decision-oriented so another agent can act without rediscovering the same context.

## Repository Boundaries

- Do not touch or commit untracked reference folders while completing root-crate planning work.
- Prefer README updates for user-facing commands and `overview.md` updates for maintainer architecture context.
- Keep future web, transport, persistence, and embedded work framed as adapters around `BattleshipApp`.
