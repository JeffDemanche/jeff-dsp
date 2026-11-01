# Architecture Decision Records

Record **product** and **engineering architecture** decisions here so humans and agents share the same history.

## When to write an ADR

Write one when you choose or change:

- Repo layout or build tooling
- Plugin formats, identity (manufacturer/plugin codes), or packaging
- FFI / language boundary strategy
- Shared module or crate design that others will depend on
- Dependency vendors or version pins that affect the stack
- Product scope that constrains engineering (e.g. “effects only for now”)

Skip ADRs for routine bugfixes, local renames, or purely cosmetic UI tweaks.

## File naming

```text
NNNN-short-kebab-title.md
```

Use the next zero-padded number (`0001`, `0002`, …).

## Template

```markdown
# NNNN Title

- Status: Proposed | Accepted | Superseded by NNNN
- Date: YYYY-MM-DD

## Context

What problem or force made a decision necessary?

## Decision

What did we choose?

## Consequences

What becomes easier, harder, or off-limits? Follow-ups?
```

## Rules for agents

1. Prefer updating an existing ADR’s status (e.g. Superseded) over silently contradicting it.
2. Keep ADRs concise; link to `docs/architecture/` for how-to detail.
3. Ship the ADR in the same change set as the code when possible.
