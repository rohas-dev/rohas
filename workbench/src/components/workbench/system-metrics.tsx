"use client";

import { useEffect, useState, useRef } from "react";
import { Cpu, MemoryStick } from "lucide-react";
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  Legend,
  ResponsiveContainer,
} from "recharts";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { fetchSystemMetrics } from "@/lib/workbench-data";

interface DataPoint {
  time: string;
  ram: number;
  cpu: number;
}

const MAX_DATA_POINTS = 60; // 2 minutes at 2-second intervals

export function SystemMetrics() {
  const [data, setData] = useState<DataPoint[]>([]);
  const [currentRam, setCurrentRam] = useState({ used: 0, total: 0, percentage: 0 });
  const [currentCpu, setCurrentCpu] = useState(0);
  const timeRef = useRef(0);

  useEffect(() => {
    const updateMetrics = async () => {
      try {
        const metrics = await fetchSystemMetrics();
        
        if (metrics) {
          const ramPercentage = Math.round(metrics.ram.percentage);
          
          setCurrentRam({
            used: metrics.ram.used_mb,
            total: metrics.ram.total_mb,
            percentage: ramPercentage,
          });
          setCurrentCpu(Math.round(metrics.cpu));

          // Add new data point
          timeRef.current += 2;
          const minutes = Math.floor(timeRef.current / 60);
          const seconds = timeRef.current % 60;
          const timeLabel = `${minutes}:${seconds.toString().padStart(2, "0")}`;

          setData((prev) => {
            const newData = [
              ...prev,
              {
                time: timeLabel,
                ram: ramPercentage,
                cpu: Math.round(metrics.cpu),
              },
            ];

            // Keep only the last MAX_DATA_POINTS
            if (newData.length > MAX_DATA_POINTS) {
              return newData.slice(-MAX_DATA_POINTS);
            }
            return newData;
          });
        }
      } catch (error) {
        console.error("Failed to update system metrics:", error);
      }
    };

    // Initialize with first data point
    updateMetrics();

    // Update every 2 seconds
    const interval = setInterval(updateMetrics, 2000);

    return () => clearInterval(interval);
  }, []);

  const formatBytes = (mb: number): string => {
    if (mb >= 1024) {
      return `${(mb / 1024).toFixed(1)} GB`;
    }
    return `${mb} MB`;
  };

  return (
    <Card className="lg:col-span-2">
      <CardHeader>
        <CardTitle>System Resources</CardTitle>
        <CardDescription>Real-time RAM and CPU usage monitoring</CardDescription>
      </CardHeader>
      <CardContent className="space-y-6">
        {/* Current Stats */}
        <div className="grid grid-cols-2 gap-4">
          <div className="space-y-2">
            <div className="flex items-center gap-2 text-sm">
              <MemoryStick className="h-4 w-4 text-muted-foreground" />
              <span className="font-medium">RAM</span>
            </div>
            <div className="text-2xl font-semibold">{currentRam.percentage}%</div>
            <div className="text-xs text-muted-foreground">
              {formatBytes(currentRam.used)} / {formatBytes(currentRam.total)}
            </div>
          </div>
          <div className="space-y-2">
            <div className="flex items-center gap-2 text-sm">
              <Cpu className="h-4 w-4 text-muted-foreground" />
              <span className="font-medium">CPU</span>
            </div>
            <div className="text-2xl font-semibold">{currentCpu}%</div>
            <div className="text-xs text-muted-foreground">
              {100 - currentCpu}% idle
            </div>
          </div>
        </div>

        {/* Chart */}
        <div className="h-[300px] w-full">
          <ResponsiveContainer width="100%" height="100%">
            <LineChart
              data={data}
              margin={{ top: 5, right: 10, left: 0, bottom: 0 }}
            >
              <CartesianGrid strokeDasharray="3 3" className="stroke-muted" />
              <XAxis
                dataKey="time"
                className="text-xs"
                tick={{ fill: "hsl(var(--muted-foreground))" }}
                interval={Math.floor(MAX_DATA_POINTS / 6)}
              />
              <YAxis
                domain={[0, 100]}
                className="text-xs"
                tick={{ fill: "hsl(var(--muted-foreground))" }}
                label={{ value: "Usage %", angle: -90, position: "insideLeft" }}
              />
              <Tooltip
                contentStyle={{
                  backgroundColor: "hsl(var(--card))",
                  border: "1px solid hsl(var(--border))",
                  borderRadius: "6px",
                }}
                labelStyle={{ color: "hsl(var(--foreground))" }}
              />
              <Legend />
              <Line
                type="monotone"
                dataKey="ram"
                stroke="#3b82f6"
                strokeWidth={2}
                dot={false}
                name="RAM %"
                activeDot={{ r: 4 }}
              />
              <Line
                type="monotone"
                dataKey="cpu"
                stroke="#ef4444"
                strokeWidth={2}
                dot={false}
                name="CPU %"
                activeDot={{ r: 4 }}
              />
            </LineChart>
          </ResponsiveContainer>
        </div>
      </CardContent>
    </Card>
  );
}

