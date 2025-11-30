"use client";

import type { ReactNode } from "react";
import { useEffect, useRef, useState } from "react";

import { ActivitySquare, AlertTriangle, Clock3, Globe, Zap, Calendar, Webhook } from "lucide-react";

import { Card, CardContent, CardDescription, CardHeader } from "@/components/ui/card";
import { TraceExplorer } from "@/components/workbench/trace-explorer";
import { TracingLogs } from "@/components/workbench/tracing-logs";
import { fetchTraceData, pollTraceData, type TraceRecord } from "@/lib/workbench-data";
import { formatNumber } from "@/lib/utils";

export default function TracingPage() {
  const [traces, setTraces] = useState<TraceRecord[]>([]);
  const [isPolling, setIsPolling] = useState(true);
  const lastTraceIdRef = useRef<string | undefined>(undefined);

  // Initial load
  useEffect(() => {
    fetchTraceData().then((initialTraces) => {
      setTraces(initialTraces);
      lastTraceIdRef.current = initialTraces[0]?.id;
    });
  }, []);

  // Long polling for new traces
  useEffect(() => {
    if (!isPolling) return;

    let mounted = true;

    const poll = async () => {
      if (!mounted || !isPolling) return;

      try {
        const newTraces = await pollTraceData(lastTraceIdRef.current, 30);
        if (!mounted) return;

        if (newTraces.length > 0) {
          setTraces((prev) => {
            // Prepend new traces (no limit - show all traces from storage)
            const existingIds = new Set(prev.map(t => t.id));
            const uniqueNewTraces = newTraces.filter(t => !existingIds.has(t.id));
            return [...uniqueNewTraces, ...prev];
          });
          // Update lastTraceId to the most recent trace
          lastTraceIdRef.current = newTraces[0]?.id;
        }
      } catch (error) {
        console.error("Polling error:", error);
        // Continue polling even on error
      }

      if (mounted && isPolling) {
        // Immediately start next poll
        poll();
      }
    };

    poll();

    return () => {
      mounted = false;
    };
  }, [isPolling]);

  const stats = summarizeTraces(traces);

  return (
    <div className="space-y-8">
      <section className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
        <StatCard
          icon={<ActivitySquare className="h-4 w-4 text-muted-foreground" />}
          label="Total traces"
          value={formatNumber(stats.total)}
          description="Real-time trace collection"
        />
        <StatCard
          icon={<Clock3 className="h-4 w-4 text-muted-foreground" />}
          label="Avg duration"
          value={`${stats.avgDuration.toFixed(2)}s`}
          description="Mean execution time"
        />
        <StatCard
          icon={<ActivitySquare className="h-4 w-4 text-muted-foreground" />}
          label="Active"
          value={formatNumber(stats.running)}
          description="Currently running traces"
        />
        <StatCard
          icon={<AlertTriangle className="h-4 w-4 text-muted-foreground" />}
          label="Failures"
          value={formatNumber(stats.failed)}
          description="Failed executions"
        />
      </section>

      {stats.byType && (stats.byType.api > 0 || stats.byType.event > 0 || stats.byType.cron > 0 || stats.byType.websocket > 0) && (
        <section className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
          <StatCard
            icon={<Globe className="h-4 w-4 text-blue-500" />}
            label="API traces"
            value={formatNumber(stats.byType.api)}
            description="HTTP API requests"
          />
          <StatCard
            icon={<Zap className="h-4 w-4 text-yellow-500" />}
            label="Event traces"
            value={formatNumber(stats.byType.event)}
            description="Event processing"
          />
          <StatCard
            icon={<Calendar className="h-4 w-4 text-purple-500" />}
            label="Cron traces"
            value={formatNumber(stats.byType.cron)}
            description="Scheduled jobs"
          />
          <StatCard
            icon={<Webhook className="h-4 w-4 text-green-500" />}
            label="WebSocket traces"
            value={formatNumber(stats.byType.websocket)}
            description="WebSocket connections"
          />
        </section>
      )}

      <section>
        <TraceExplorer traces={traces} />
      </section>

      <section>
        <TracingLogs />
      </section>
    </div>
  );
}

function summarizeTraces(traces: Awaited<ReturnType<typeof fetchTraceData>>) {
  if (traces.length === 0) {
    return { 
      total: 0, 
      avgDuration: 0, 
      running: 0, 
      failed: 0,
      byType: { api: 0, event: 0, cron: 0, websocket: 0 }
    };
  }
  const total = traces.length;
  const running = traces.filter((trace) => trace.status === "running").length;
  const failed = traces.filter((trace) => trace.status === "failed").length;
  const avgDuration =
    traces.reduce((acc, trace) => acc + trace.durationMs, 0) / traces.length / 1000;
  
  const byType = {
    api: traces.filter((t) => t.entryType === "api").length,
    event: traces.filter((t) => t.entryType === "event").length,
    cron: traces.filter((t) => t.entryType === "cron").length,
    websocket: traces.filter((t) => t.entryType === "websocket").length,
  };

  return { total, avgDuration, running, failed, byType };
}

function StatCard({
  icon,
  label,
  value,
  description,
}: {
  icon: ReactNode;
  label: string;
  value: string;
  description: string;
}) {
  return (
    <Card>
      <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
        <CardDescription>{label}</CardDescription>
        {icon}
      </CardHeader>
      <CardContent>
        <div className="text-2xl font-semibold">{value}</div>
        <p className="text-sm text-muted-foreground">{description}</p>
      </CardContent>
    </Card>
  );
}

