import path from "node:path";
import { promises as fs } from "node:fs";
import { parse } from "toml";

export type ProjectConfig = {
  project?: {
    name?: string;
    version?: string;
    language?: string;
  };
  server?: {
    host?: string;
    port?: number;
  };
  adapter?: Record<string, unknown>;
};

export type SchemaBucket = {
  name: string;
  files: ProjectFile[];
};

export type ProjectFile = {
  name: string;
  relativePath: string;
  size: number;
  content?: string;
};

export type HandlerBucket = SchemaBucket;

export type ProjectSnapshot = {
  root: string;
  config?: ProjectConfig;
  schema: {
    total: number;
    buckets: SchemaBucket[];
  };
  handlers: {
    total: number;
    buckets: HandlerBucket[];
  };
};

export async function loadProjectSnapshot(): Promise<ProjectSnapshot> {
  const root = await resolveProjectRoot();
  const [config, schema, handlers] = await Promise.all([
    readProjectConfig(root),
    readSchemaBuckets(root),
    readHandlerBuckets(root),
  ]);

  return {
    root,
    config,
    schema,
    handlers,
  };
}

async function resolveProjectRoot(): Promise<string> {
  const override = process.env.ROHAS_PROJECT_ROOT;
  if (override) {
    return path.resolve(override);
  }

  let current = process.cwd();
  const limit = path.parse(current).root;

  while (true) {
    const candidate = path.join(current, "config", "rohas.toml");
    if (await fileExists(candidate)) {
      return current;
    }

    if (current === limit) {
      break;
    }

    current = path.dirname(current);
  }

  const repoRoot = path.resolve(process.cwd(), "..");
  const sampleCandidates = [
    path.join(repoRoot, "examples", "hello-world"),
    path.join(repoRoot, "examples", "python-hello-world"),
  ];

  for (const candidate of sampleCandidates) {
    const configPath = path.join(candidate, "config", "rohas.toml");
    if (await fileExists(configPath)) {
      return candidate;
    }
  }

  return repoRoot;
}

async function readProjectConfig(root: string): Promise<ProjectConfig | undefined> {
  try {
    const configPath = path.join(root, "config", "rohas.toml");
    if (!(await fileExists(configPath))) {
      return undefined;
    }
    const raw = await fs.readFile(configPath, "utf-8");
    return parse(raw) as ProjectConfig;
  } catch (error) {
    console.warn("Failed to read project config:", error);
    return undefined;
  }
}

async function readSchemaBuckets(root: string) {
  const schemaDir = path.join(root, "schema");
  const buckets = await collectBuckets(root, schemaDir, true, ".ro");

  return {
    total: buckets.reduce((acc, bucket) => acc + bucket.files.length, 0),
    buckets,
  };
}

async function readHandlerBuckets(root: string) {
  const handlersDir = path.join(root, "src", "handlers");
  const buckets = await collectBuckets(root, handlersDir, true, ".py", ".ts", ".js");

  return {
    total: buckets.reduce((acc, bucket) => acc + bucket.files.length, 0),
    buckets,
  };
}

async function collectBuckets(
  projectRoot: string,
  baseDir: string,
  captureContent: boolean,
  ...extensions: string[]
): Promise<SchemaBucket[]> {
  if (!(await dirExists(baseDir))) {
    return [];
  }

  const entries = await fs.readdir(baseDir, { withFileTypes: true });
  const buckets: SchemaBucket[] = [];

  for (const entry of entries) {
    if (!entry.isDirectory()) continue;
    const bucketDir = path.join(baseDir, entry.name);
    const files = await collectFiles(projectRoot, bucketDir, captureContent, extensions);
    if (files.length === 0) continue;

    buckets.push({
      name: entry.name,
      files,
    });
  }

  return buckets.sort((a, b) => b.files.length - a.files.length);
}

async function collectFiles(
  projectRoot: string,
  dir: string,
  captureContent: boolean,
  extensions: string[],
): Promise<ProjectFile[]> {
  if (!(await dirExists(dir))) {
    return [];
  }

  const entries = await fs.readdir(dir, { withFileTypes: true });
  const files: ProjectFile[] = [];

  for (const entry of entries) {
    const absolute = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      files.push(...(await collectFiles(projectRoot, absolute, captureContent, extensions)));
      continue;
    }

    if (!extensions.some((ext) => entry.name.toLowerCase().endsWith(ext))) {
      continue;
    }

    const stats = await fs.stat(absolute);
    const content = captureContent ? await fs.readFile(absolute, "utf-8") : undefined;
    files.push({
      name: entry.name,
      relativePath: path.relative(projectRoot, absolute),
      size: stats.size,
      content,
    });
  }

  return files;
}

async function fileExists(file: string) {
  try {
    await fs.access(file);
    return true;
  } catch {
    return false;
  }
}

async function dirExists(dir: string) {
  try {
    const stats = await fs.stat(dir);
    return stats.isDirectory();
  } catch {
    return false;
  }
}

