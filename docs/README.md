# Documentation

This directory is the shared knowledge base for **humans and AI agents**.

| Path | Contents |
|------|----------|
| [`architecture/`](architecture/) | How the system is structured and how to extend it |
| [`decisions/`](decisions/) | Architecture Decision Records (ADRs) |

## For agents

1. Read this page and [`architecture/overview.md`](architecture/overview.md) before structural changes.
2. When you make a product or engineering architecture decision, add an ADR under [`decisions/`](decisions/) and update architecture docs if the system shape changed.
3. Follow [`AGENTS.md`](../AGENTS.md) and the Cursor rule `.cursor/rules/document-decisions.mdc`.

## For humans

Start with the architecture overview, then skim ADRs for why things are the way they are. Prefer updating docs in the same change as the code.
