"use client";

import type { ReactNode } from "react";
import { useEffect, useRef, useState } from "react";

import { ActivitySquare, AlertTriangle, Clock3 } from "lucide-react";

import { Card, CardContent, CardDescription, CardHeader } from "@/components/ui/card";
import { TraceExplorer } from "@/components/workbench/trace-explorer";
import { TracingLogs } from "@/components/workbench/tracing-logs";
import { fetchTraceData, pollTraceData, type TraceRecord } from "@/lib/workbench-data";

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
            // Prepend new traces and keep only the most recent 100
            const combined = [...newTraces, ...prev];
            return combined.slice(0, 100);
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
          value={stats.total.toString()}
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
          value={stats.running.toString()}
          description="Currently running traces"
        />
        <StatCard
          icon={<AlertTriangle className="h-4 w-4 text-muted-foreground" />}
          label="Failures"
          value={stats.failed.toString()}
          description="Failed executions"
        />
      </section>

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
    return { total: 0, avgDuration: 0, running: 0, failed: 0 };
  }
  const total = traces.length;
  const running = traces.filter((trace) => trace.status === "running").length;
  const failed = traces.filter((trace) => trace.status === "failed").length;
  const avgDuration =
    traces.reduce((acc, trace) => acc + trace.durationMs, 0) / traces.length / 1000;

  return { total, avgDuration, running, failed };
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

