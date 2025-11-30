"use client";

import { useEffect, useRef, useState, useMemo, useCallback, memo } from "react";
import { AlertCircle, Info, XCircle, Zap, Filter, X, Search, Play, Pause, Maximize2, Minimize2 } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { fetchTracingLogs, pollTracingLogs, type TracingLogEntry } from "@/lib/workbench-data";
import { cn, formatNumber } from "@/lib/utils";

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

interface LogWatchProps {
  open: boolean;
  onClose: () => void;
}

export function LogWatch({ open, onClose }: LogWatchProps) {
  const [logs, setLogs] = useState<TracingLogEntry[]>([]);
  const [levelFilter, setLevelFilter] = useState<string>("all");
  const [isPolling, setIsPolling] = useState(true);
  const [showFilters, setShowFilters] = useState(true);
  const [searchQuery, setSearchQuery] = useState("");
  const [startTime, setStartTime] = useState("");
  const [endTime, setEndTime] = useState("");
  const [targetFilter, setTargetFilter] = useState("");
  const [spanNameFilter, setSpanNameFilter] = useState("");
  const [autoScroll, setAutoScroll] = useState(true);
  const [isLoadingMore, setIsLoadingMore] = useState(false);
  const [hasMoreLogs, setHasMoreLogs] = useState(true);
  const [debouncedSearchQuery, setDebouncedSearchQuery] = useState("");
  const scrollAreaRef = useRef<HTMLDivElement>(null);
  const oldestTimestampRef = useRef<string | undefined>(undefined);
  const lastTimestampRef = useRef<string | undefined>(undefined);
  const POLLING_INTERVAL = 3000; // 3 seconds - reduced frequency to prevent overload
  const MAX_LOGS = 1000; // Maximum logs to keep in memory (reduced for performance)
  const RENDER_LIMIT = 400; // Maximum logs to render in DOM

  useEffect(() => {
    const timer = setTimeout(() => {
      setDebouncedSearchQuery(searchQuery);
    }, 300);
    return () => clearTimeout(timer);
  }, [searchQuery]);


  useEffect(() => {
    if (!open) return;
    
    fetchTracingLogs(500, levelFilter !== "all" ? levelFilter : undefined).then((initialLogs) => {
      if (initialLogs.length > 0) {
        setLogs(initialLogs);
        lastTimestampRef.current = initialLogs[0]?.timestamp;
        oldestTimestampRef.current = initialLogs[initialLogs.length - 1]?.timestamp;
        setHasMoreLogs(initialLogs.length >= 500);
      }
    });
  }, [levelFilter, open]);

  useEffect(() => {
    if (!open || !isPolling) return;

    let isPollingActive = true;

    const poll = async () => {
      if (!isPollingActive || !open) return;

      try {
        const newLogs = await pollTracingLogs(
          lastTimestampRef.current,
          levelFilter !== "all" ? levelFilter : undefined,
          1 
        );

        if (!isPollingActive || !open) return;

        if (newLogs.length > 0) {
          setLogs((prev) => {
            const combined = [...newLogs, ...prev];
            const limited = combined.slice(0, MAX_LOGS);
            lastTimestampRef.current = limited[0]?.timestamp;
            return limited;
          });
      
          if (autoScroll && scrollAreaRef.current) {
            const scrollContainer = scrollAreaRef.current.querySelector('[data-radix-scroll-area-viewport]') as HTMLElement;
            if (scrollContainer) {
              scrollContainer.scrollTop = 0;
            }
          }
        }
      } catch (error) {
        console.error("Polling error:", error);
      }

      if (isPollingActive && open) {
        setTimeout(poll, POLLING_INTERVAL);
      }
    };

    const timeoutId = setTimeout(poll, POLLING_INTERVAL);

    return () => {
      isPollingActive = false;
      clearTimeout(timeoutId);
    };
  }, [isPolling, levelFilter, open, autoScroll]);

  const loadMoreLogs = useCallback(async () => {
    if (isLoadingMore || !hasMoreLogs) return;

    setIsLoadingMore(true);
    try {
      const moreLogs = await fetchTracingLogs(500, levelFilter !== "all" ? levelFilter : undefined);
      
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
          setHasMoreLogs(olderLogs.length >= 500);
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
  }, [isLoadingMore, hasMoreLogs, levelFilter]);

  const scrollTimeoutRef = useRef<NodeJS.Timeout | null>(null);
  const handleScroll = useCallback((e: React.UIEvent<HTMLDivElement>) => {
    if (scrollTimeoutRef.current) return; // Throttle scroll events
    
    scrollTimeoutRef.current = setTimeout(() => {
      const target = e.currentTarget;
      const scrollContainer = target.querySelector('[data-radix-scroll-area-viewport]') as HTMLElement;
      if (!scrollContainer) {
        scrollTimeoutRef.current = null;
        return;
      }

      const { scrollTop, scrollHeight, clientHeight } = scrollContainer;
      const isNearBottom = scrollHeight - scrollTop - clientHeight < 300; // 300px threshold

      if (isNearBottom && hasMoreLogs && !isLoadingMore && !autoScroll) {
        loadMoreLogs();
      }
      scrollTimeoutRef.current = null;
    }, 200); // Throttle to 200ms
  }, [hasMoreLogs, isLoadingMore, autoScroll, loadMoreLogs]);

  useEffect(() => {
    return () => {
      if (scrollTimeoutRef.current) {
        clearTimeout(scrollTimeoutRef.current);
      }
    };
  }, []);
 
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

  if (!open) return null;

  return (
    <div className="fixed inset-0 z-50 bg-background/95 backdrop-blur-sm">
      <div className="flex h-full flex-col">
        {/* Header */}
        <div className="border-b bg-background/95 backdrop-blur-sm">
          <div className="flex h-16 items-center justify-between px-6">
            <div className="flex items-center gap-4">
              <div>
                <h2 className="text-lg font-semibold">Log Watch</h2>
                <p className="text-sm text-muted-foreground">
                  Real-time structured logs from tokio-rs/tracing
                  {hasActiveFilters && (
                    <Badge variant="secondary" className="ml-2">
                      {formatNumber(filteredLogs.length)} of {formatNumber(logs.length)} logs
                    </Badge>
                  )}
                </p>
              </div>
            </div>
            <div className="flex items-center gap-2">
              <div className="relative">
                <Search className="absolute left-2 top-1/2 transform -translate-y-1/2 h-4 w-4 text-muted-foreground" />
                <Input
                  type="text"
                  placeholder="Search logs..."
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  className="pl-8 h-9 w-64"
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
                variant="outline"
                size="sm"
                onClick={() => setAutoScroll(!autoScroll)}
                title={autoScroll ? "Disable auto-scroll" : "Enable auto-scroll"}
              >
                {autoScroll ? <Minimize2 className="h-4 w-4" /> : <Maximize2 className="h-4 w-4" />}
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
              <Button variant="outline" size="sm" onClick={onClose}>
                <X className="h-4 w-4" />
              </Button>
            </div>
          </div>

          {/* Filter Panel */}
          {showFilters && (
            <div className="border-t bg-muted/20 p-4 space-y-4">
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
        </div>

        <div className="flex-1 overflow-hidden">
          <ScrollArea ref={scrollAreaRef} className="h-full" onScroll={handleScroll}>
            {filteredLogs.length === 0 ? (
              <div className="flex flex-col items-center justify-center h-full text-center p-8">
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
                  {filteredLogs.slice(0, RENDER_LIMIT).map((log) => (
                    <LogEntry key={`${log.timestamp}-${log.target}-${log.message?.slice(0, 20)}`} log={log} searchQuery={debouncedSearchQuery} />
                  ))}
                  {filteredLogs.length > RENDER_LIMIT && (
                    <div className="p-4 text-center text-sm text-muted-foreground">
                      Showing first {formatNumber(RENDER_LIMIT)} of {formatNumber(filteredLogs.length)} filtered logs. Use filters to narrow down results.
                    </div>
                  )}
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
        </div>
      </div>
    </div>
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
            <span className={cn("text-xs font-medium", colorClass)}>{log.level.toUpperCase()}</span>
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

