"use client";

import { useEffect, useRef, useState } from "react";
import { AlertCircle, Info, XCircle, Zap } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { fetchTracingLogs, pollTracingLogs, type TracingLogEntry } from "@/lib/workbench-data";

const levelIcons = {
  error: <XCircle className="h-4 w-4 text-red-500" />,
  warn: <AlertCircle className="h-4 w-4 text-amber-500" />,
  info: <Info className="h-4 w-4 text-blue-500" />,
  debug: <Zap className="h-4 w-4 text-muted-foreground" />,
  trace: <Zap className="h-4 w-4 text-muted-foreground" />,
};

const levelColors = {
  error: "text-red-600 dark:text-red-400",
  warn: "text-amber-600 dark:text-amber-400",
  info: "text-blue-600 dark:text-blue-400",
  debug: "text-muted-foreground",
  trace: "text-muted-foreground",
};

export function TracingLogs() {
  const [logs, setLogs] = useState<TracingLogEntry[]>([]);
  const [levelFilter, setLevelFilter] = useState<string>("all");
  const [isPolling, setIsPolling] = useState(true);
  const lastTimestampRef = useRef<string | undefined>(undefined);

  useEffect(() => {
    fetchTracingLogs(100, levelFilter !== "all" ? levelFilter : undefined).then((initialLogs) => {
      setLogs(initialLogs);
      lastTimestampRef.current = initialLogs[0]?.timestamp;
    });
  }, [levelFilter]);

  useEffect(() => {
    if (!isPolling) return;

    let mounted = true;

    const poll = async () => {
      if (!mounted || !isPolling) return;

      try {
        const newLogs = await pollTracingLogs(
          lastTimestampRef.current,
          levelFilter !== "all" ? levelFilter : undefined,
          30
        );
        if (!mounted) return;

        if (newLogs.length > 0) {
          setLogs((prev) => {
            const combined = [...newLogs, ...prev];
            return combined.slice(0, 200);
          });
          lastTimestampRef.current = newLogs[0]?.timestamp;
        }
      } catch (error) {
        console.error("Polling error:", error);
      }

      if (mounted && isPolling) {
        poll();
      }
    };

    poll();

    return () => {
      mounted = false;
    };
  }, [isPolling, levelFilter]);

  const filteredLogs = logs.filter((log) => levelFilter === "all" || log.level === levelFilter);

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <div>
            <CardTitle>Tracing Logs</CardTitle>
            <CardDescription>Real-time structured logs from tokio-rs/tracing</CardDescription>
          </div>
          <div className="flex items-center gap-2">
            <Tabs value={levelFilter} onValueChange={setLevelFilter}>
              <TabsList>
                <TabsTrigger value="all">All</TabsTrigger>
                <TabsTrigger value="error">Error</TabsTrigger>
                <TabsTrigger value="warn">Warn</TabsTrigger>
                <TabsTrigger value="info">Info</TabsTrigger>
                <TabsTrigger value="debug">Debug</TabsTrigger>
              </TabsList>
            </Tabs>
          </div>
        </div>
      </CardHeader>
      <CardContent className="p-0">
        <ScrollArea className="h-[500px]">
          {filteredLogs.length === 0 ? (
            <div className="p-8 text-center text-sm text-muted-foreground">
              No logs available. Make API requests to see tracing logs.
            </div>
          ) : (
            <div className="divide-y">
              {filteredLogs.map((log, index) => (
                <LogEntry key={`${log.timestamp}-${index}`} log={log} />
              ))}
            </div>
          )}
        </ScrollArea>
      </CardContent>
    </Card>
  );
}

function LogEntry({ log }: { log: TracingLogEntry }) {
  const level = log.level.toLowerCase() as keyof typeof levelIcons;
  const icon = levelIcons[level] || <Info className="h-4 w-4" />;
  const colorClass = levelColors[level] || "text-muted-foreground";
  const timestamp = new Date(log.timestamp).toLocaleTimeString();

  return (
    <div className="p-4 hover:bg-muted/50 transition-colors">
      <div className="flex items-start gap-3">
        <div className="mt-0.5">{icon}</div>
        <div className="flex-1 min-w-0 space-y-1">
          <div className="flex items-center gap-2 flex-wrap">
            <span className={`text-xs font-medium ${colorClass}`}>{log.level.toUpperCase()}</span>
            <span className="text-xs text-muted-foreground">{timestamp}</span>
            {log.file && log.line && (
              <span className="text-xs text-muted-foreground font-mono">
                {log.file}:{log.line}
              </span>
            )}
            {log.span_name && (
              <Badge variant="outline" className="text-xs">
                {log.span_name}
              </Badge>
            )}
            <span className="text-xs text-muted-foreground">{log.target}</span>
          </div>
          <p className="text-sm text-foreground">{log.message || "—"}</p>
          {(Object.keys(log.fields).length > 0 || Object.keys(log.span_fields).length > 0) && (
            <div className="mt-2 space-y-1">
              {Object.entries(log.fields).map(([key, value]) => (
                <div key={key} className="text-xs text-muted-foreground">
                  <span className="font-mono">{key}</span>: <span>{value}</span>
                </div>
              ))}
              {Object.entries(log.span_fields).map(([key, value]) => (
                <div key={key} className="text-xs text-muted-foreground">
                  <span className="font-mono font-semibold">{key}</span>: <span>{value}</span>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

