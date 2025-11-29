import path from "node:path";
import { promises as fs } from "node:fs";
import { parse } from "toml";

export type WorkspaceCrate = {
  name: string;
  relativePath: string;
  version?: string;
  description?: string;
  kind: "core" | "adapter" | "tooling";
};

const WORKSPACE_ROOT = path.resolve(process.cwd(), "..");
const CRATES_ROOT = path.join(WORKSPACE_ROOT, "crates");

export async function getWorkspaceCrates(): Promise<WorkspaceCrate[]> {
  const crates: WorkspaceCrate[] = [];
  await walkCrates(CRATES_ROOT, crates);
  crates.sort((a, b) => a.name.localeCompare(b.name));
  return crates;
}

async function walkCrates(dir: string, crates: WorkspaceCrate[]) {
  const entries = await fs.readdir(dir, { withFileTypes: true });

  for (const entry of entries) {
    if (!entry.isDirectory()) continue;
    const absolute = path.join(dir, entry.name);
    const cargoToml = path.join(absolute, "Cargo.toml");

    if (await exists(cargoToml)) {
      const crate = await readCrateMetadata(cargoToml);
      if (crate) {
        crates.push({
          ...crate,
          relativePath: path.relative(WORKSPACE_ROOT, absolute),
          kind: classifyCrate(absolute),
        });
      }
    } else {
      await walkCrates(absolute, crates);
    }
  }
}

async function readCrateMetadata(
  cargoPath: string,
): Promise<Omit<WorkspaceCrate, "relativePath" | "kind"> | null> {
  try {
    const file = await fs.readFile(cargoPath, "utf-8");
    const data = parse(file) as {
      package?: {
        name?: string;
        version?: unknown;
        description?: unknown;
      };
    };
    if (!data.package?.name) return null;

    return {
      name: data.package.name,
      version: stringifyVersion(data.package.version),
      description: typeof data.package.description === "string" ? data.package.description : undefined,
    };
  } catch (error) {
    console.warn(`Failed to parse ${cargoPath}:`, error);
    return null;
  }
}

function classifyCrate(absPath: string): WorkspaceCrate["kind"] {
  const normalized = absPath.replace(/\\/g, "/");
  if (normalized.includes("adapters")) return "adapter";
  if (normalized.includes("cli") || normalized.includes("dev-server")) {
    return "tooling";
  }
  return "core";
}

function stringifyVersion(value: unknown): string | undefined {
  if (typeof value === "string") {
    return value;
  }

  if (
    typeof value === "object" &&
    value !== null &&
    "workspace" in (value as Record<string, unknown>)
  ) {
    return "workspace";
  }

  return undefined;
}

async function exists(file: string) {
  try {
    await fs.access(file);
    return true;
  } catch {
    return false;
  }
}

