# Workbench 

Rohas Workbench is a Next.js dashboard that pairs generated Rohas projects with a modern shadcn/ui front-end. 

## Stack

- Next.js 16 / React 19 (App Router)
- Tailwind CSS 3.4 + shadcn/ui primitives (`src/components/ui/*`)
- next-themes for dark/light modes
- `toml` parser to read Cargo metadata straight from disk

## Getting started

Use the `rohas` CLI to boot both the engine and the workbench from any project that has a `config/rohas.toml`:

```bash
# Fastest feedback loop: Next.js dev server with hot reload
cargo run --bin rohas -- dev --workbench-dev

# Closer to production: builds the workbench and runs `next start`
cargo run --bin rohas -- dev --workbench
```

Then open `http://localhost:4401` in your browser. The workbench will automatically connect to the Rohas engine


## Project layout & routes

- `src/app/(workbench)` – Overview, Schemas, Schema Graph, Workflows, Tracing, CLI, Settings routes
- `src/app/(workbench)/workflows/[slug]` – interactive workflow viewer
- `src/app/(workbench)/tracing` – schema-derived trace explorer
- `src/components/ui` – reusable shadcn primitives (button, card, tabs, etc.)
- `src/components/workbench` – domain widgets (sidebar, header, schema browser, workflow tools)
- `src/stores/workbench-store.ts` – Zustand store for cross-route search & navigation state
- `src/lib/project.ts` / `src/lib/workbench-data.ts` – filesystem helpers that discover schema, handler, and workflow assets

