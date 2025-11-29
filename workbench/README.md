# Workbench

Rohas Workbench is a Next.js dashboard that pairs generated Rohas projects with a modern shadcn/ui front-end. It automatically detects the nearest `config/rohas.toml`, parses your local schemas/handlers, and surfaces health signals without needing a backend service.

## Stack

- Next.js 16 / React 19 (App Router)
- Tailwind CSS 3.4 + shadcn/ui primitives (`src/components/ui/*`)
- next-themes for dark/light modes
- `toml` parser to read Cargo metadata straight from disk

## Getting started

```bash
pnpm install
pnpm dev
```

Visit http://localhost:3000 to view the dashboard. Updating schema files under `./schema` or handlers under `./src/handlers` immediately updates the inventory, because Workbench reads directly from disk on every request.

## Scripts

```bash
pnpm lint       # next lint (core-web-vitals rules)
pnpm typecheck  # tsc --noEmit
pnpm format     # prettier on src/**/*.{ts,tsx}
pnpm build && pnpm start
```

## Project layout & routes

- `src/app/(workbench)` – Overview, Schemas, Schema Graph, Workflows, Tracing, CLI, Settings routes
- `src/app/(workbench)/workflows/[slug]` – interactive workflow viewer
- `src/app/(workbench)/tracing` – schema-derived trace explorer
- `src/components/ui` – reusable shadcn primitives (button, card, tabs, etc.)
- `src/components/workbench` – domain widgets (sidebar, header, schema browser, workflow tools)
- `src/stores/workbench-store.ts` – Zustand store for cross-route search & navigation state
- `src/lib/project.ts` / `src/lib/workbench-data.ts` – filesystem helpers that discover schema, handler, and workflow assets

Because the UI executes inside the same repo as the generated Rohas project, no network calls are required—Workbench renders everything from local state, which keeps the feedback loop fast even when offline.

## Project root detection

Workbench walks up from the app directory until it finds `config/rohas.toml`, which becomes the project root. Override this detection with:

```bash
ROHAS_PROJECT_ROOT=/absolute/path/to/project pnpm dev
```

The override is especially helpful when running the dashboard from a globally installed bundle or when the project structure deviates from the default generator layout.
