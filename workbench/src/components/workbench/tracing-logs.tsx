"use client";

import { useEffect, useRef, useState, useMemo, useCallback, memo } from "react";
import { AlertCircle, Info, XCircle, Zap, Filter, X, Search, Play, Pause, ExternalLink } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { fetchTracingLogs, pollTracingLogs, type TracingLogEntry } from "@/lib/workbench-data";
import { formatNumber } from "@/lib/utils";
import { LogWatch } from "./log-watch";

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
  const [showFilters, setShowFilters] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const [startTime, setStartTime] = useState("");
  const [endTime, setEndTime] = useState("");
  const [targetFilter, setTargetFilter] = useState("");
  const [spanNameFilter, setSpanNameFilter] = useState("");
  const [showLogWatch, setShowLogWatch] = useState(false);
  const [isLoadingMore, setIsLoadingMore] = useState(false);
  const [hasMoreLogs, setHasMoreLogs] = useState(true);
  const scrollAreaRef = useRef<HTMLDivElement>(null);
  const oldestTimestampRef = useRef<string | undefined>(undefined);
  const lastTimestampRef = useRef<string | undefined>(undefined);
  const POLLING_INTERVAL = 2000; // 2 seconds
  const MAX_LOGS = 500; // Maximum logs to keep in memory

  // Initial load
  useEffect(() => {
    fetchTracingLogs(100, levelFilter !== "all" ? levelFilter : undefined).then((initialLogs) => {
      if (initialLogs.length > 0) {
        setLogs(initialLogs);
        lastTimestampRef.current = initialLogs[0]?.timestamp;
        oldestTimestampRef.current = initialLogs[initialLogs.length - 1]?.timestamp;
        setHasMoreLogs(initialLogs.length >= 100);
      }
    });
  }, [levelFilter]);
 
  useEffect(() => {
    if (!isPolling) return;

    const intervalId = setInterval(async () => {
      try {
        const newLogs = await pollTracingLogs(
          lastTimestampRef.current,
          levelFilter !== "all" ? levelFilter : undefined,
          1  
        );

        if (newLogs.length > 0) {
          setLogs((prev) => {
            const combined = [...newLogs, ...prev];
            const limited = combined.slice(0, MAX_LOGS);
            lastTimestampRef.current = limited[0]?.timestamp;
            return limited;
          });
        }
      } catch (error) {
        console.error("Polling error:", error);
      }
    }, POLLING_INTERVAL);

    return () => clearInterval(intervalId);
  }, [isPolling, levelFilter]);

 
  const loadMoreLogs = async () => {
    if (isLoadingMore || !hasMoreLogs) return;

    setIsLoadingMore(true);
    try {
      const moreLogs = await fetchTracingLogs(100, levelFilter !== "all" ? levelFilter : undefined);
      
      if (moreLogs.length > 0) {
        const olderLogs = moreLogs.filter(log => {
          if (!oldestTimestampRef.current) return true;
          return new Date(log.timestamp).getTime() < new Date(oldestTimestampRef.current).getTime();
        });

        if (olderLogs.length > 0) {
          setLogs((prev) => {
            const combined = [...prev, ...olderLogs];
            const limited = combined.slice(-MAX_LOGS);
            oldestTimestampRef.current = limited[limited.length - 1]?.timestamp;
            return limited;
          });
          setHasMoreLogs(olderLogs.length >= 100);
        } else {
          setHasMoreLogs(false);
        }
      } else {
        setHasMoreLogs(false);
      }
    } catch (error) {
      console.error("Failed to load more logs:", error);
    } finally {
      setIsLoadingMore(false);
    }
  };

  const handleScroll = (e: React.UIEvent<HTMLDivElement>) => {
    const target = e.currentTarget;
    const scrollContainer = target.querySelector('[data-radix-scroll-area-viewport]') as HTMLElement;
    if (!scrollContainer) return;

    const { scrollTop, scrollHeight, clientHeight } = scrollContainer;
    const isNearBottom = scrollHeight - scrollTop - clientHeight < 100; // 100px threshold

    if (isNearBottom && hasMoreLogs && !isLoadingMore) {
      loadMoreLogs();
    }
  };

  const [debouncedSearchQuery, setDebouncedSearchQuery] = useState("");
  useEffect(() => {
    const timer = setTimeout(() => {
      setDebouncedSearchQuery(searchQuery);
    }, 300);  
    return () => clearTimeout(timer);
  }, [searchQuery]);

  const filteredLogs = useMemo(() => {
    let filtered = logs;

    if (levelFilter !== "all") {
      filtered = filtered.filter((log) => log.level.toLowerCase() === levelFilter.toLowerCase());
    }

    if (debouncedSearchQuery.trim()) {
      const query = debouncedSearchQuery.toLowerCase();
      filtered = filtered.filter((log) =>
        log.message?.toLowerCase().includes(query) ||
        log.target?.toLowerCase().includes(query) ||
        log.span_name?.toLowerCase().includes(query) ||
        Object.entries(log.fields).some(([key, value]) =>
          key.toLowerCase().includes(query) || String(value).toLowerCase().includes(query)
        ) ||
        Object.entries(log.span_fields).some(([key, value]) =>
          key.toLowerCase().includes(query) || String(value).toLowerCase().includes(query)
        )
      );
    }

    if (targetFilter.trim()) {
      filtered = filtered.filter((log) =>
        log.target?.toLowerCase().includes(targetFilter.toLowerCase())
      );
    }

    if (spanNameFilter.trim()) {
      filtered = filtered.filter((log) =>
        log.span_name?.toLowerCase().includes(spanNameFilter.toLowerCase())
      );
    }

    if (startTime) {
      const start = new Date(startTime).getTime();
      filtered = filtered.filter((log) => {
        const logTime = new Date(log.timestamp).getTime();
        return logTime >= start;
      });
    }

    if (endTime) {
      const end = new Date(endTime).getTime();
      filtered = filtered.filter((log) => {
        const logTime = new Date(log.timestamp).getTime();
        return logTime <= end;
      });
    }

    return filtered;
  }, [logs, levelFilter, debouncedSearchQuery, targetFilter, spanNameFilter, startTime, endTime]);

  const hasActiveFilters =
    searchQuery !== "" ||
    targetFilter !== "" ||
    spanNameFilter !== "" ||
    startTime !== "" ||
    endTime !== "" ||
    levelFilter !== "all";

  const clearFilters = () => {
    setSearchQuery("");
    setTargetFilter("");
    setSpanNameFilter("");
    setStartTime("");
    setEndTime("");
    setLevelFilter("all");
  };

  const setQuickTimeRange = (range: "1h" | "24h" | "7d" | "30d") => {
    const now = new Date();
    let start: Date;

    switch (range) {
      case "1h":
        start = new Date(now.getTime() - 60 * 60 * 1000);
        break;
      case "24h":
        start = new Date(now.getTime() - 24 * 60 * 60 * 1000);
        break;
      case "7d":
        start = new Date(now.getTime() - 7 * 24 * 60 * 60 * 1000);
        break;
      case "30d":
        start = new Date(now.getTime() - 30 * 24 * 60 * 60 * 1000);
        break;
    }

    const formatDateTime = (date: Date) => {
      const year = date.getFullYear();
      const month = String(date.getMonth() + 1).padStart(2, "0");
      const day = String(date.getDate()).padStart(2, "0");
      const hours = String(date.getHours()).padStart(2, "0");
      const minutes = String(date.getMinutes()).padStart(2, "0");
      return `${year}-${month}-${day}T${hours}:${minutes}`;
    };

    setStartTime(formatDateTime(start));
    setEndTime(formatDateTime(now));
  };

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <div>
            <CardTitle>Tracing Logs</CardTitle>
            <CardDescription>
              Real-time structured logs from tokio-rs/tracing
              {hasActiveFilters && (
                <Badge variant="secondary" className="ml-2">
                  {formatNumber(filteredLogs.length)} of {formatNumber(logs.length)} logs
                </Badge>
              )}
            </CardDescription>
          </div>
          <div className="flex items-center gap-2">
            <div className="relative">
              <Search className="absolute left-2 top-1/2 transform -translate-y-1/2 h-4 w-4 text-muted-foreground" />
              <Input
                type="text"
                placeholder="Search logs..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="pl-8 h-8 w-48 text-xs"
              />
            </div>
            <Button
              variant={showFilters ? "default" : "outline"}
              size="sm"
              onClick={() => setShowFilters(!showFilters)}
            >
              <Filter className="h-4 w-4 mr-2" />
              Filters
              {hasActiveFilters && (
                <Badge variant="secondary" className="ml-2 h-4 px-1 text-[10px]">
                  {levelFilter !== "all" ? "L" : ""}
                  {(startTime || endTime) ? "D" : ""}
                  {searchQuery ? "S" : ""}
                  {(targetFilter || spanNameFilter) ? "F" : ""}
                </Badge>
              )}
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={() => setIsPolling(!isPolling)}
              title={isPolling ? "Pause log updates" : "Resume log updates"}
            >
              {isPolling ? <Pause className="h-4 w-4" /> : <Play className="h-4 w-4" />}
            </Button>
            <Button
              variant="default"
              size="sm"
              onClick={() => setShowLogWatch(true)}
              title="Open full-page log watch"
            >
              <ExternalLink className="h-4 w-4 mr-2" />
              Log Watch
            </Button>
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

        {showFilters && (
          <div className="mt-4 p-4 border rounded-lg bg-muted/20 space-y-4">
            <div className="flex items-center justify-between">
              <h4 className="text-sm font-semibold">Filters</h4>
              {hasActiveFilters && (
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={clearFilters}
                  className="h-7 text-xs"
                >
                  <X className="h-3 w-3 mr-1" />
                  Clear
                </Button>
              )}
            </div>

            <div className="grid grid-cols-2 gap-4">
              <div>
                <label className="text-xs font-medium text-muted-foreground mb-2 block">
                  Target Filter
                </label>
                <Input
                  type="text"
                  placeholder="Filter by target..."
                  value={targetFilter}
                  onChange={(e) => setTargetFilter(e.target.value)}
                  className="h-8 text-xs"
                />
              </div>
              <div>
                <label className="text-xs font-medium text-muted-foreground mb-2 block">
                  Span Name Filter
                </label>
                <Input
                  type="text"
                  placeholder="Filter by span name..."
                  value={spanNameFilter}
                  onChange={(e) => setSpanNameFilter(e.target.value)}
                  className="h-8 text-xs"
                />
              </div>
            </div>

            <div>
              <label className="text-xs font-medium text-muted-foreground mb-2 block">
                Time Range
              </label>
              <div className="flex flex-wrap gap-2 mb-3">
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => setQuickTimeRange("1h")}
                  className="h-7 text-xs"
                >
                  Last Hour
                </Button>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => setQuickTimeRange("24h")}
                  className="h-7 text-xs"
                >
                  Last 24 Hours
                </Button>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => setQuickTimeRange("7d")}
                  className="h-7 text-xs"
                >
                  Last 7 Days
                </Button>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => setQuickTimeRange("30d")}
                  className="h-7 text-xs"
                >
                  Last 30 Days
                </Button>
              </div>
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="text-xs font-medium text-muted-foreground mb-2 block">
                    Start Time
                  </label>
                  <Input
                    type="datetime-local"
                    value={startTime}
                    onChange={(e) => setStartTime(e.target.value)}
                    className="h-8 text-xs"
                  />
                </div>
                <div>
                  <label className="text-xs font-medium text-muted-foreground mb-2 block">
                    End Time
                  </label>
                  <Input
                    type="datetime-local"
                    value={endTime}
                    onChange={(e) => setEndTime(e.target.value)}
                    className="h-8 text-xs"
                  />
                </div>
              </div>
            </div>
          </div>
        )}
      </CardHeader>
      <CardContent className="p-0">
        <ScrollArea ref={scrollAreaRef} className="h-[500px]" onScroll={handleScroll}>
          {filteredLogs.length === 0 ? (
            <div className="flex flex-col items-center justify-center h-[400px] text-center p-8">
              <Filter className="h-12 w-12 text-muted-foreground mb-4" />
              <h3 className="text-lg font-semibold mb-2">
                {hasActiveFilters ? "No logs match the filters" : "No logs available"}
              </h3>
              <p className="text-sm text-muted-foreground mb-4">
                {hasActiveFilters
                  ? "Try adjusting your filter criteria or clear filters to see all logs."
                  : "Make API requests or trigger events to see tracing logs."}
              </p>
              {hasActiveFilters && (
                <Button variant="outline" size="sm" onClick={clearFilters}>
                  Clear Filters
                </Button>
              )}
            </div>
            ) : (
              <>
                <div className="divide-y">
                  {filteredLogs.map((log, index) => (
                    <LogEntry key={`${log.timestamp}-${index}`} log={log} searchQuery={searchQuery} />
                  ))}
                </div>
                {isLoadingMore && (
                  <div className="p-4 text-center text-sm text-muted-foreground">
                    Loading more logs...
                  </div>
                )}
                {!hasMoreLogs && filteredLogs.length > 0 && (
                  <div className="p-4 text-center text-sm text-muted-foreground">
                    No more logs to load
                  </div>
                )}
              </>
            )}
        </ScrollArea>
      </CardContent>
      <LogWatch open={showLogWatch} onClose={() => setShowLogWatch(false)} />
    </Card>
  );
}

const LogEntry = memo(function LogEntry({ log, searchQuery = "" }: { log: TracingLogEntry; searchQuery?: string }) {
  const level = log.level.toLowerCase() as keyof typeof levelIcons;
  const icon = levelIcons[level] || <Info className="h-4 w-4" />;
  const colorClass = levelColors[level] || "text-muted-foreground";
  const timestamp = useMemo(() => new Date(log.timestamp).toLocaleTimeString(), [log.timestamp]);

  const highlightText = useCallback((text: string, query: string) => {
    if (!query.trim()) return text;
    const escapedQuery = query.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    const parts = text.split(new RegExp(`(${escapedQuery})`, "gi"));
    return parts.map((part, i) =>
      part.toLowerCase() === query.toLowerCase() ? (
        <mark key={i} className="bg-yellow-200 dark:bg-yellow-900 px-0.5 rounded">
          {part}
        </mark>
      ) : (
        part
      )
    );
  }, []);

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
          <p className="text-sm text-foreground">
            {log.message ? highlightText(log.message, searchQuery) : "—"}
          </p>
          {(Object.keys(log.fields).length > 0 || Object.keys(log.span_fields).length > 0) && (
            <div className="mt-2 space-y-1">
              {Object.entries(log.fields).map(([key, value]) => (
                <div key={key} className="text-xs text-muted-foreground">
                  <span className="font-mono">{highlightText(key, searchQuery)}</span>:{" "}
                  <span>{highlightText(String(value), searchQuery)}</span>
                </div>
              ))}
              {Object.entries(log.span_fields).map(([key, value]) => (
                <div key={key} className="text-xs text-muted-foreground">
                  <span className="font-mono font-semibold">{highlightText(key, searchQuery)}</span>
                  : <span>{highlightText(String(value), searchQuery)}</span>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}, (prevProps, nextProps) => {
  return (
    prevProps.log.timestamp === nextProps.log.timestamp &&
    prevProps.log.message === nextProps.log.message &&
    prevProps.searchQuery === nextProps.searchQuery
  );
});

