import { apiRequest, setApiBaseUrl } from "@/lib/api";
import type { ActivityItem, EntityRow } from "@/types/workbench";

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

export type WorkbenchData = {
  root: string;
  config?: ProjectConfig;
  schemaTotal: number;
  schemaBucketCount: number;
  handlerTotal: number;
  schemaRows: EntityRow[];
  handlerRows: EntityRow[];
  activity: ActivityItem[];
};

export type TraceStatus = "success" | "failed" | "running";

export type TriggeredEventInfo = {
  event_name: string;
  timestamp: string;
  duration_ms: number;
};

export type TraceStep = {
  name: string;
  path: string;
  bucket: string;
  handler_name: string;
  duration_ms: number;
  success: boolean;
  error?: string | null;
  timestamp: string;
  triggered_events?: TriggeredEventInfo[];
};

type ApiTraceRecord = {
  id: string;
  entry_point: string;
  entry_type: string;
  bucket: string;
  status: string;
  duration_ms: number;
  started_at: string;
  completed_at?: string | null;
  steps: TraceStep[];
  error?: string | null;
  metadata?: Record<string, string>;
};

export type TraceRecord = {
  id: string;
  entryPoint: string;
  entryType: "api" | "event" | "cron" | "websocket";
  bucket: string;
  status: TraceStatus;
  durationMs: number;
  startedAt: string;
  completedAt?: string | null;
  steps: TraceStep[];
  error?: string | null;
  metadata?: Record<string, string>;
};

export async function fetchWorkbenchData(): Promise<WorkbenchData> {
  try {
    const data = await apiRequest<ApiWorkbenchData>("/api/workbench/data");
    
    // Update API base URL from config if available
    if (data.config?.server?.host && data.config?.server?.port) {
      const apiBase = `http://${data.config.server.host}:${data.config.server.port}`;
      setApiBaseUrl(apiBase);
    }
    
    // Map snake_case from API to camelCase for backward compatibility
    return {
      root: data.root,
      config: data.config,
      schemaTotal: data.schema_total,
      schemaBucketCount: data.schema_bucket_count,
      handlerTotal: data.handler_total,
      schemaRows: data.schema_rows,
      handlerRows: data.handler_rows,
      activity: data.activity,
    };
  } catch (error) {
    console.error("Failed to fetch workbench data:", error);
    // Return empty data structure on error
    return {
      root: ".",
      schemaTotal: 0,
      schemaBucketCount: 0,
      handlerTotal: 0,
      schemaRows: [],
      handlerRows: [],
      activity: [
        {
          id: "error",
          title: "Failed to load data",
          description: error instanceof Error ? error.message : "Unknown error",
          timestamp: "just now",
        },
      ],
    };
  }
}

type ApiWorkbenchData = {
  root: string;
  config?: ProjectConfig;
  schema_total: number;
  schema_bucket_count: number;
  handler_total: number;
  schema_rows: EntityRow[];
  handler_rows: EntityRow[];
  activity: ActivityItem[];
};

export async function fetchTraceData(): Promise<TraceRecord[]> {
  try {
    const traces = await apiRequest<ApiTraceRecord[]>("/api/workbench/traces");
    // Map snake_case from API to camelCase
    return traces.map((trace) => ({
      id: trace.id,
      entryPoint: trace.entry_point,
      entryType: trace.entry_type as "api" | "event" | "cron" | "websocket",
      bucket: trace.bucket,
      status: trace.status as TraceStatus,
      durationMs: trace.duration_ms,
      startedAt: trace.started_at,
      completedAt: trace.completed_at,
      error: trace.error,
      metadata: trace.metadata,
      steps: trace.steps,
    }));
  } catch (error) {
    console.error("Failed to fetch trace data:", error);
    return [];
  }
}

export async function pollTraceData(since?: string, timeout: number = 30): Promise<TraceRecord[]> {
  try {
    const params = new URLSearchParams();
    if (since) {
      params.append("since", since);
    }
    params.append("timeout", timeout.toString());
    
    const traces = await apiRequest<ApiTraceRecord[]>(`/api/workbench/traces/poll?${params.toString()}`);
    // Map snake_case from API to camelCase
    return traces.map((trace) => ({
      id: trace.id,
      entryPoint: trace.entry_point,
      entryType: trace.entry_type as "api" | "event" | "cron" | "websocket",
      bucket: trace.bucket,
      status: trace.status as TraceStatus,
      durationMs: trace.duration_ms,
      startedAt: trace.started_at,
      completedAt: trace.completed_at,
      error: trace.error,
      metadata: trace.metadata,
      steps: trace.steps,
    }));
  } catch (error) {
    console.error("Failed to poll trace data:", error);
    return [];
  }
}

export type TracingLogEntry = {
  timestamp: string;
  level: string;
  target: string;
  message: string;
  fields: Record<string, string>;
  span_name?: string;
  span_fields: Record<string, string>;
  file?: string | null;
  line?: number | null;
};

export async function fetchTracingLogs(limit?: number, level?: string): Promise<TracingLogEntry[]> {
  try {
    const params = new URLSearchParams();
    if (limit) {
      params.append("limit", limit.toString());
    }
    if (level) {
      params.append("level", level);
    }
    
    const query = params.toString();
    const endpoint = query ? `/api/workbench/logs?${query}` : "/api/workbench/logs";
    return await apiRequest<TracingLogEntry[]>(endpoint);
  } catch (error) {
    console.error("Failed to fetch tracing logs:", error);
    return [];
  }
}

export async function pollTracingLogs(since?: string, level?: string, timeout: number = 30): Promise<TracingLogEntry[]> {
  try {
    const params = new URLSearchParams();
    if (since) {
      params.append("since", since);
    }
    if (level) {
      params.append("level", level);
    }
    params.append("timeout", timeout.toString());
    
    return await apiRequest<TracingLogEntry[]>(`/api/workbench/logs/poll?${params.toString()}`);
  } catch (error) {
    console.error("Failed to poll tracing logs:", error);
    return [];
  }
}
 
export type SchemaGraphNode = {
  id: string;
  label: string;
  bucket: string;
  path: string;
  type: "schema" | "handler";
  source?: string;
};

export type SchemaGraphEdge = {
  id: string;
  source: string;
  target: string;
  relation: string;
};

export type SchemaGraph = {
  root: string;
  nodes: SchemaGraphNode[];
  edges: SchemaGraphEdge[];
};

type ApiSchemaGraphNode = {
  id: string;
  label: string;
  bucket: string;
  path: string;
  type: string; // Rust serializes node_type as "type" in JSON
  source?: string;
};

type ApiSchemaGraph = {
  root: string;
  nodes: ApiSchemaGraphNode[];
  edges: SchemaGraphEdge[];
};

export async function fetchSchemaGraph(): Promise<SchemaGraph> {
  try {
    const graph = await apiRequest<ApiSchemaGraph>("/api/workbench/schema-graph");
    // The API returns "type" field (renamed from node_type in Rust)
    return {
      root: graph.root,
      nodes: graph.nodes.map((node) => ({
        id: node.id,
        label: node.label,
        bucket: node.bucket,
        path: node.path,
        type: node.type as "schema" | "handler",
        source: node.source,
      })),
      edges: graph.edges,
    };
  } catch (error) {
    console.error("Failed to fetch schema graph:", error);
    return {
      root: ".",
      nodes: [],
      edges: [],
    };
  }
}

export type ApiEndpoint = {
  name: string;
  method: string;
  path: string;
  body: string | null;
  response: string;
  triggers: string[];
  middlewares: string[];
};

export type WebSocketEndpoint = {
  name: string;
  path: string;
  message: string | null;
  on_connect: string[];
  on_message: string[];
  on_disconnect: string[];
  triggers: string[];
  broadcast: boolean;
  middlewares: string[];
};

export type CronJob = {
  name: string;
  schedule: string;
  triggers: string[];
};

export type EventEndpoint = {
  name: string;
  payload: string;
  handlers: string[];
  triggers: string[];
};

export type EndpointsData = {
  apis: ApiEndpoint[];
  websockets: WebSocketEndpoint[];
  crons: CronJob[];
  events: EventEndpoint[];
};

export async function fetchEndpoints(): Promise<EndpointsData> {
  try {
    const data = await apiRequest<EndpointsData>("/api/workbench/endpoints");
    return data;
  } catch (error) {
    console.error("Failed to fetch endpoints:", error);
    return {
      apis: [],
      websockets: [],
      crons: [],
      events: [],
    };
  }
}

export async function fetchTypeSchema(typeName: string | null): Promise<unknown | null> {
  if (!typeName) {
    return null;
  }

  try {
    const schema = await apiRequest<unknown>(`/api/workbench/types/${encodeURIComponent(typeName)}`);
    return schema;
  } catch (error) {
    console.error(`Failed to fetch schema for type ${typeName}:`, error);
    return null;
  }
}
