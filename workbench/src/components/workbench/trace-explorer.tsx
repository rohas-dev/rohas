"use client";

import "@xyflow/react/dist/style.css";

import { useMemo, useState } from "react";
import { ChevronDown, ChevronRight, Clock, CheckCircle2, XCircle, Zap, Globe, Webhook, Calendar } from "lucide-react";
import {
  ReactFlow,
  ReactFlowProvider,
  useNodesState,
  useEdgesState,
  Handle,
  Position,
  NodeProps,
  Background,
  Controls,
  type Node,
  type Edge,
} from "@xyflow/react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { ScrollArea } from "@/components/ui/scroll-area";
import { cn } from "@/lib/utils";
import type { TraceRecord, TraceStep } from "@/lib/workbench-data";
import { fetchTracingLogs, type TracingLogEntry } from "@/lib/workbench-data";

const statusVariants: Record<
  TraceRecord["status"],
  { label: string; className: string }
> = {
  success: { label: "Success", className: "bg-emerald-500/15 text-emerald-600" },
  failed: { label: "Failed", className: "bg-red-500/15 text-red-600" },
  running: { label: "Running", className: "bg-amber-500/15 text-amber-600" },
};

export function TraceExplorer({ traces }: { traces: TraceRecord[] }) {
  const [selectedTraceId, setSelectedTraceId] = useState(traces[0]?.id ?? null);
  const [expandedTraces, setExpandedTraces] = useState<Set<string>>(new Set());
  const selectedTrace = useMemo(
    () => traces.find((trace) => trace.id === selectedTraceId) ?? traces[0] ?? null,
    [selectedTraceId, traces],
  );

  const toggleExpand = (traceId: string) => {
    setExpandedTraces((prev) => {
      const next = new Set(prev);
      if (next.has(traceId)) {
        next.delete(traceId);
      } else {
        next.add(traceId);
      }
      return next;
    });
  };

  if (traces.length === 0) {
    return <p className="text-sm text-muted-foreground">No traces derived from schema yet.</p>;
  }

  return (
    <div className="grid gap-6 lg:grid-cols-[minmax(0,1fr)_400px]">
      <Card>
        <CardHeader>
          <CardTitle>Trace inventory</CardTitle>
          <CardDescription>Click a trace to inspect its details. Expand to see full route.</CardDescription>
        </CardHeader>
        <CardContent className="p-0">
          <ScrollArea className="h-[600px]">
            <div className="divide-y">
              {traces.map((trace) => {
                const statusMeta = statusVariants[trace.status];
                const isExpanded = expandedTraces.has(trace.id);
                const isSelected = selectedTrace?.id === trace.id;
                
                return (
                  <div
                    key={trace.id}
                    className={cn(
                      "transition-colors",
                      isSelected && "bg-muted/50"
                    )}
                  >
                    <div
                      onClick={() => setSelectedTraceId(trace.id)}
                      className="cursor-pointer p-4 hover:bg-muted/40 transition-colors"
                    >
                      <div className="flex items-start justify-between gap-4">
                        <div className="flex-1 min-w-0">
                          <div className="flex items-center gap-2 mb-1">
                            <Button
                              variant="ghost"
                              size="sm"
                              className="h-6 w-6 p-0"
                              onClick={(e) => {
                                e.stopPropagation();
                                toggleExpand(trace.id);
                              }}
                            >
                              {isExpanded ? (
                                <ChevronDown className="h-4 w-4" />
                              ) : (
                                <ChevronRight className="h-4 w-4" />
                              )}
                            </Button>
                            <p className="font-medium text-foreground">{trace.entryPoint}</p>
                            <span
                              className={`rounded-full px-2 py-0.5 text-xs font-semibold ${statusMeta.className}`}
                            >
                              {statusMeta.label}
                            </span>
                          </div>
                          <p className="text-xs text-muted-foreground ml-8">{trace.bucket}</p>
                          <div className="flex items-center gap-4 mt-2 ml-8 text-xs text-muted-foreground">
                            <span className="flex items-center gap-1">
                              <Clock className="h-3 w-3" />
                              {trace.durationMs}ms
                            </span>
                            <span>{new Date(trace.startedAt).toLocaleTimeString()}</span>
                          </div>
                        </div>
                      </div>
                    </div>
                    
                    {isExpanded && (
                      <TraceDetails trace={trace} />
                    )}
                  </div>
                );
              })}
            </div>
          </ScrollArea>
        </CardContent>
      </Card>

      {selectedTrace && (
        <Card>
          <CardHeader>
            <CardTitle>{selectedTrace.entryPoint}</CardTitle>
            <CardDescription>
              Started {new Date(selectedTrace.startedAt).toLocaleTimeString()} •{" "}
              {selectedTrace.durationMs}ms
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4 text-sm">
            <div className="flex items-center gap-2">
              <Badge variant="outline" className="capitalize">
                {selectedTrace.bucket}
              </Badge>
              {selectedTrace.error && (
                <Badge variant="destructive" className="text-xs">
                  Error
                </Badge>
              )}
            </div>
            
            {selectedTrace.error && (
              <div className="rounded-lg border border-red-500/20 bg-red-500/5 p-3">
                <div className="flex items-start gap-2">
                  <XCircle className="h-4 w-4 text-red-500 mt-0.5" />
                  <div className="flex-1">
                    <p className="font-medium text-red-600 dark:text-red-400">Error</p>
                    <p className="text-xs text-red-600/80 dark:text-red-400/80 mt-1">{selectedTrace.error}</p>
                  </div>
                </div>
              </div>
            )}

            {selectedTrace.metadata && Object.keys(selectedTrace.metadata).length > 0 && (
              <div>
                <p className="text-xs font-medium text-muted-foreground mb-2">Metadata</p>
                <div className="space-y-1">
                  {Object.entries(selectedTrace.metadata).map(([key, value]) => (
                    <div key={key} className="text-xs text-muted-foreground">
                      <span className="font-mono">{key}</span>: <span>{value}</span>
                    </div>
                  ))}
                </div>
              </div>
            )}

            <div>
              <p className="text-xs font-medium text-muted-foreground mb-2">Execution Route</p>
              <div className="space-y-2">
                {selectedTrace.steps.map((step, index) => (
                  <TraceStepDetail key={`${step.handler_name}-${index}`} step={step} index={index} />
                ))}
              </div>
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  );
}

function TraceDetails({ trace }: { trace: TraceRecord }) {
  const [logs, setLogs] = useState<TracingLogEntry[]>([]);
  const [loadingLogs, setLoadingLogs] = useState(false);

  const loadLogs = async () => {
    if (loadingLogs) return;
    setLoadingLogs(true);
    try {
      const allLogs = await fetchTracingLogs(200);
      const traceLogs = allLogs.filter((log) => {
        const logTime = new Date(log.timestamp).getTime();
        const traceStart = new Date(trace.startedAt).getTime();
        const traceEnd = trace.completedAt 
          ? new Date(trace.completedAt).getTime()
          : Date.now();
        
        return logTime >= traceStart && logTime <= traceEnd;
      });
      setLogs(traceLogs);
    } catch (error) {
      console.error("Failed to load logs:", error);
    } finally {
      setLoadingLogs(false);
    }
  };

  return (
    <div className="border-t bg-muted/20 p-4 space-y-4">
      <div>
        <p className="text-xs font-medium text-muted-foreground mb-2">Execution Flow</p>
        <ReactFlowProvider>
          <TraceFlowVisualization trace={trace} />
        </ReactFlowProvider>
      </div>
      
      <div>
        <p className="text-xs font-medium text-muted-foreground mb-2">Execution Route Details</p>
        <div className="space-y-2">
          {trace.steps.map((step, index) => (
            <TraceStepDetail key={`${step.handler_name}-${index}`} step={step} index={index} />
          ))}
        </div>
      </div>

      <div>
        <Button
          variant="outline"
          size="sm"
          onClick={loadLogs}
          disabled={loadingLogs}
          className="w-full"
        >
          {loadingLogs ? "Loading..." : "View Tracing Logs"}
        </Button>
        {logs.length > 0 && (
          <div className="mt-3 space-y-2 max-h-[300px] overflow-y-auto">
            {logs.map((log, idx) => (
              <div key={idx} className="text-xs p-2 rounded border bg-background">
                <div className="flex items-center gap-2 mb-1 flex-wrap">
                  <span className={cn(
                    "font-medium",
                    log.level === "error" && "text-red-600",
                    log.level === "warn" && "text-amber-600",
                    log.level === "info" && "text-blue-600"
                  )}>
                    {log.level.toUpperCase()}
                  </span>
                  <span className="text-muted-foreground">
                    {new Date(log.timestamp).toLocaleTimeString()}
                  </span>
                  {log.file && log.line && (
                    <span className="text-muted-foreground font-mono text-[10px]">
                      {log.file}:{log.line}
                    </span>
                  )}
                  {log.span_name && (
                    <Badge variant="outline" className="text-xs">
                      {log.span_name}
                    </Badge>
                  )}
                </div>
                <p className="text-foreground">{log.message}</p>
                {log.fields && Object.keys(log.fields).length > 0 && (
                  <div className="mt-1 space-y-0.5">
                    {Object.entries(log.fields).map(([key, value]) => (
                      <div key={key} className="text-muted-foreground">
                        <span className="font-mono">{key}</span>: {value}
                      </div>
                    ))}
                  </div>
                )}
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
 
function RoundNode({ data }: NodeProps) {
  const nodeData = data as {
    label: string;
    subtitle?: string;
    success?: boolean;
    icon?: React.ComponentType<{ size?: number; className?: string }>;
    duration?: number;
    isTrigger?: boolean;
  };

  const Icon = nodeData.icon;
  const isSuccess = nodeData.success !== false;
  const isTrigger = nodeData.isTrigger || false;
   
  const sizeClass = isTrigger 
    ? "min-w-[80px] min-h-[80px] p-2" 
    : "min-w-[120px] min-h-[120px] p-4";
  
  const bgColor = isTrigger
    ? "bg-blue-500/20 border-blue-500/40"
    : isSuccess 
      ? "bg-emerald-500/20 border-emerald-500/40" 
      : "bg-red-500/20 border-red-500/40";
  const textColor = isTrigger
    ? "text-blue-600 dark:text-blue-400"
    : isSuccess 
      ? "text-emerald-600 dark:text-emerald-400" 
      : "text-red-600 dark:text-red-400";

  return (
    <div className="relative">
      <Handle type="target" position={Position.Left} />
      <div
        className={cn(
          "flex flex-col items-center justify-center rounded-full border-2",
          sizeClass,
          bgColor
        )}
      >
        {Icon && (
          <Icon size={isTrigger ? 16 : 24} className={cn("mb-1", textColor)} />
        )}
        <div className="text-center">
          <div className={cn(isTrigger ? "text-xs" : "text-sm", "font-semibold mb-1", textColor)}>
            {nodeData.label}
          </div>
          {nodeData.subtitle && (
            <div className={cn("text-[10px] text-muted-foreground", !isTrigger && "mb-1")}>
              {nodeData.subtitle}
            </div>
          )}
          {nodeData.duration !== undefined && (
            <div className={cn("text-muted-foreground", isTrigger ? "text-[10px]" : "text-xs")}>
              {nodeData.duration}ms
            </div>
          )}
        </div>
      </div>
      <Handle type="source" position={Position.Right} />
    </div>
  );
}

const traceNodeTypes = {
  round: RoundNode,
};

function TraceFlowVisualization({ trace }: { trace: TraceRecord }) {
  const entryTypeIcon = {
    api: Globe,
    event: Zap,
    cron: Calendar,
    websocket: Webhook,
  }[trace.entryType || "api"] || Globe;

  const nodes: Node[] = useMemo(() => {
    const nodeList: Node[] = [];
    let xPosition = 0;
    const yPosition = 150;
    const horizontalSpacing = 200;
    const triggerOffsetY = 80;

    nodeList.push({
      id: "entry",
      type: "round",
      position: { x: xPosition, y: yPosition },
      data: {
        label: trace.entryPoint,
        subtitle: trace.entryType?.toUpperCase() || "ENTRY",
        success: trace.status === "success",
        icon: entryTypeIcon,
        duration: trace.durationMs,
      },
    });

    xPosition += horizontalSpacing;

    trace.steps.forEach((step, index) => {
      nodeList.push({
        id: `step-${index}`,
        type: "round",
        position: { x: xPosition, y: yPosition },
        data: {
          label: step.name || step.handler_name,
          subtitle: step.bucket || step.path,
          success: step.success,
          duration: step.duration_ms,
        },
      });

      const triggers = step.triggered_events || [];
      if (triggers.length > 0) {
        triggers.forEach((trigger, triggerIndex) => {
          const triggerY = yPosition + triggerOffsetY + (triggerIndex * 60);
          const triggerTime = new Date(trigger.timestamp).toLocaleTimeString();
          nodeList.push({
            id: `trigger-${index}-${triggerIndex}`,
            type: "round",
            position: { x: xPosition, y: triggerY },
            data: {
              label: trigger.event_name,
              subtitle: `${triggerTime} (${trigger.duration_ms}ms)`,
              success: true,
              icon: Zap,
              isTrigger: true,
              duration: trigger.duration_ms,
            },
          });
        });
      }

      xPosition += horizontalSpacing;
    });

    return nodeList;
  }, [trace, entryTypeIcon]);

  const edges: Edge[] = useMemo(() => {
    const edgeList: Edge[] = [];
    
    if (trace.steps.length > 0) {
      edgeList.push({
        id: "entry-step-0",
        source: "entry",
        target: "step-0",
        animated: true,
        style: { stroke: "hsl(var(--primary))", strokeWidth: 2 },
      });
    }

    for (let i = 0; i < trace.steps.length; i++) {
      if (i < trace.steps.length - 1) {
        edgeList.push({
          id: `step-${i}-step-${i + 1}`,
          source: `step-${i}`,
          target: `step-${i + 1}`,
          animated: true,
          style: { 
            stroke: trace.steps[i].success 
              ? "hsl(var(--primary))" 
              : "hsl(var(--destructive))",
            strokeWidth: 2,
          },
        });
      }

      const triggers = trace.steps[i].triggered_events || [];
      triggers.forEach((trigger, triggerIndex) => {
        edgeList.push({
          id: `step-${i}-trigger-${i}-${triggerIndex}`,
          source: `step-${i}`,
          target: `trigger-${i}-${triggerIndex}`,
          animated: true,
          style: { 
            stroke: "hsl(var(--muted-foreground))",
            strokeWidth: 1.5,
            strokeDasharray: "5,5",
          },
        });
      });
    }

    return edgeList;
  }, [trace.steps]);

  const [flowNodes, setNodes, onNodesChange] = useNodesState(nodes);
  const [flowEdges, setEdges, onEdgesChange] = useEdgesState(edges);

  useMemo(() => {
    setNodes(nodes);
    setEdges(edges);
  }, [nodes, edges, setNodes, setEdges]);

  return (
    <div className="h-[300px] w-full rounded-lg border overflow-hidden bg-background">
      <ReactFlow
        nodes={flowNodes}
        edges={flowEdges}
        nodeTypes={traceNodeTypes}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        fitView
        minZoom={0.5}
        maxZoom={1.5}
        defaultViewport={{ x: 0, y: 0, zoom: 0.8 }}
      >
        <Background gap={16} size={1} />
        <Controls />
      </ReactFlow>
    </div>
  );
}

function TraceStepDetail({ step }: { step: TraceStep; index: number }) {
  return (
    <div className={cn(
      "rounded-lg border p-3",
      step.success ? "border-green-500/20 bg-green-500/5" : "border-red-500/20 bg-red-500/5"
    )}>
      <div className="flex items-start justify-between gap-2">
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 mb-1 flex-wrap">
            {step.success ? (
              <CheckCircle2 className="h-4 w-4 text-green-500" />
            ) : (
              <XCircle className="h-4 w-4 text-red-500" />
            )}
            <p className="font-medium text-foreground">{step.name || step.handler_name}</p>
            <Badge variant={step.success ? "default" : "destructive"} className="text-xs">
              {step.success ? "Success" : "Failed"}
            </Badge>
          </div>
          <p className="text-xs text-muted-foreground ml-6 font-mono">{step.path}</p>
          <div className="flex items-center gap-3 mt-2 ml-6 text-xs text-muted-foreground">
            <span className="flex items-center gap-1">
              <Clock className="h-3 w-3" />
              {step.duration_ms}ms
            </span>
            <span>{new Date(step.timestamp).toLocaleTimeString()}</span>
          </div>
          {step.error && (
            <div className="mt-2 ml-6 rounded border border-red-500/20 bg-red-500/5 p-2">
              <p className="text-xs font-medium text-red-600 dark:text-red-400">Error</p>
              <p className="text-xs text-red-600/80 dark:text-red-400/80 mt-1 font-mono break-all">{step.error}</p>
            </div>
          )}
          {step.triggered_events && step.triggered_events.length > 0 && (
            <div className="mt-2 ml-6 rounded border border-blue-500/20 bg-blue-500/5 p-2">
              <div className="flex items-center gap-2 mb-2">
                <Zap className="h-3 w-3 text-blue-500" />
                <p className="text-xs font-medium text-blue-600 dark:text-blue-400">Triggered Events</p>
              </div>
              <div className="space-y-1.5">
                {step.triggered_events.map((event, idx) => (
                  <div key={idx} className="flex items-center justify-between gap-2">
                    <Badge variant="outline" className="text-xs bg-blue-500/10 border-blue-500/30 text-blue-600 dark:text-blue-400">
                      {event.event_name}
                    </Badge>
                    <div className="flex items-center gap-2 text-xs text-muted-foreground">
                      <span>{new Date(event.timestamp).toLocaleTimeString()}</span>
                      <span className="text-muted-foreground/70">
                        ({event.duration_ms}ms)
                      </span>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

