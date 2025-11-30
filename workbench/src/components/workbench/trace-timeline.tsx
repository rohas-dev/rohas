"use client";

import { useMemo, useRef, useState, useEffect, useCallback } from "react";
import { ZoomIn, ZoomOut, Zap, Globe, Webhook, Calendar, Filter, X, Search, ChevronDown, ChevronRight, Play, Pause, Maximize2, Minimize2, ChevronsDownUp, ChevronsUpDown } from "lucide-react";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { cn, formatNumber } from "@/lib/utils";
import type { TraceRecord } from "@/lib/workbench-data";
import { useTimelineFiltersStore, type TraceType } from "@/stores/timeline-filters-store";

const TRACK_HEIGHT = 60;
const TRACK_LABEL_WIDTH = 220;
const MIN_PIXELS_PER_MS = 0.1;
const MAX_PIXELS_PER_MS = 10;
const COLLAPSED_TRACK_HEIGHT = 24;

const entryTypeIcons = {
  api: Globe,
  event: Zap,
  cron: Calendar,
  websocket: Webhook,
};


const formatDuration = (ms: number) => {
  if (ms < 1000) return `${ms}ms`;
  return `${(ms / 1000).toFixed(2)}s`;
};

interface TimelineBlock {
  id: string;
  label: string;
  startTime: number;
  duration: number;
  type: "trace" | "step" | "trigger";
  status?: "success" | "failed" | "running";
  entryType?: "api" | "event" | "cron" | "websocket";
  traceId: string;
  stepIndex?: number;
  triggerIndex?: number;
}

const MAX_RENDERED_TRACES = 100; // Limit rendered traces to prevent performance issues

export function TraceTimeline({ traces }: { traces: TraceRecord[] }) {
  const [scrollLeft, setScrollLeft] = useState(0);
  const [selectedBlock, setSelectedBlock] = useState<string | null>(null);
  const [playheadPosition, setPlayheadPosition] = useState<number | null>(null);
  const [isPlaying, setIsPlaying] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const [debouncedSearchQuery, setDebouncedSearchQuery] = useState("");
  const [collapsedTracks, setCollapsedTracks] = useState<Set<string>>(new Set());
  const [showMinimap, setShowMinimap] = useState(true);
  const timelineRef = useRef<HTMLDivElement>(null);
  const scrollAreaRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const timer = setTimeout(() => {
      setDebouncedSearchQuery(searchQuery);
    }, 300);
    return () => clearTimeout(timer);
  }, [searchQuery]);


  const {
    showFilters,
    selectedTypes: selectedTypesArray,
    startTime,
    endTime,
    zoom,
    setShowFilters,
    toggleType,
    setStartTime,
    setEndTime,
    setZoom,
    clearFilters,
  } = useTimelineFiltersStore();

  const selectedTypes = useMemo(() => new Set(selectedTypesArray), [selectedTypesArray]);

  const filteredTraces = useMemo(() => {
    let filtered = traces.filter((trace) => selectedTypes.has(trace.entryType));
    
    if (debouncedSearchQuery.trim()) {
      const query = debouncedSearchQuery.toLowerCase();
      filtered = filtered.filter((trace) => 
        trace.entryPoint.toLowerCase().includes(query) ||
        trace.id.toLowerCase().includes(query) ||
        trace.steps.some(step => 
          step.name?.toLowerCase().includes(query) ||
          step.handler_name.toLowerCase().includes(query)
        )
      );
    }

    if (startTime) {
      const start = new Date(startTime).getTime();
      filtered = filtered.filter((trace) => {
        const traceStart = new Date(trace.startedAt).getTime();
        return traceStart >= start;
      });
    }

    if (endTime) {
      const end = new Date(endTime).getTime();
      filtered = filtered.filter((trace) => {
        const traceStart = new Date(trace.startedAt).getTime();
        return traceStart <= end;
      });
    }


    return filtered.slice(0, MAX_RENDERED_TRACES);
  }, [traces, selectedTypes, startTime, endTime, debouncedSearchQuery]);


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

  const hasActiveFilters =
    selectedTypesArray.length < 4 || startTime !== "" || endTime !== "";

  const timelineData = useMemo(() => {
    if (filteredTraces.length === 0) {
      return {
        blocks: [],
        minTime: 0,
        maxTime: 1000,
        tracks: [],
        maxDuration: 1000,
      };
    }

    const blocks: TimelineBlock[] = [];
    let maxDuration = 0;

    filteredTraces.forEach((trace) => {
      const traceStart = new Date(trace.startedAt).getTime();
      const traceEnd = trace.completedAt
        ? new Date(trace.completedAt).getTime()
        : traceStart + trace.durationMs;
      const traceDuration = traceEnd - traceStart;
      maxDuration = Math.max(maxDuration, traceDuration);


      blocks.push({
        id: `trace-${trace.id}`,
        label: trace.entryPoint,
        startTime: 0,
        duration: traceDuration,
        type: "trace",
        status: trace.status,
        entryType: trace.entryType,
        traceId: trace.id,
      });

      trace.steps.forEach((step, stepIndex) => {
        const stepStart = new Date(step.timestamp).getTime();
        const stepDuration = step.duration_ms;
        const stepRelativeTime = stepStart - traceStart;  

        blocks.push({
          id: `step-${trace.id}-${stepIndex}`,
          label: step.name || step.handler_name,
          startTime: stepRelativeTime,  
          duration: stepDuration,
          type: "step",
          status: step.success ? "success" : "failed",
          traceId: trace.id,
          stepIndex,
        });
 
        if (step.triggered_events && step.triggered_events.length > 0) {
          step.triggered_events.forEach((trigger, triggerIndex) => {
            const triggerTime = new Date(trigger.timestamp).getTime();
            const triggerRelativeTime = triggerTime - traceStart;  
            blocks.push({
              id: `trigger-${trace.id}-${stepIndex}-${triggerIndex}`,
              label: trigger.event_name,
              startTime: triggerRelativeTime,  
              duration: trigger.duration_ms,
              type: "trigger",
              status: "success",
              traceId: trace.id,
              stepIndex,
              triggerIndex,
            });
          });
        }
      });
    });
 
    let currentY = 0;
    const tracks = filteredTraces.map((trace) => {
      const traceStart = new Date(trace.startedAt).getTime();
      const traceEnd = trace.completedAt
        ? new Date(trace.completedAt).getTime()
        : traceStart + trace.durationMs;
      const traceDuration = traceEnd - traceStart;
      const isCollapsed = collapsedTracks.has(trace.id);
      const trackHeight = isCollapsed ? COLLAPSED_TRACK_HEIGHT : TRACK_HEIGHT;
      
      const track = {
        id: trace.id,
        label: trace.entryPoint,
        entryType: trace.entryType,
        status: trace.status,
        duration: traceDuration,
        y: currentY,
        height: trackHeight,
        isCollapsed,
        blocks: blocks.filter((b) => b.traceId === trace.id),
        metadata: trace.metadata,
      };
      
      currentY += trackHeight;
      return track;
    });

    return {
      blocks,
      minTime: 0,  
      maxTime: maxDuration,  
      maxDuration,
      tracks,
    };
  }, [filteredTraces, collapsedTracks]);

  const totalDuration = timelineData.maxTime - timelineData.minTime;
  const timelineWidth = totalDuration * zoom;

  const timeMarkers = useMemo(() => {
    const markers: Array<{ time: number; label: string; position: number }> = [];
    
    let interval: number;
    if (totalDuration < 1000) {
      interval = 50; // 50ms intervals for < 1s
    } else if (totalDuration < 10000) {
      interval = 100; // 100ms intervals for < 10s
    } else if (totalDuration < 60000) {
      interval = 1000; // 1s intervals for < 1min
    } else {
      interval = 5000; // 5s intervals for longer traces
    }
    
    for (let time = 0; time <= totalDuration; time += interval) {
      const position = time * zoom;
      const label = formatDuration(time);
      markers.push({ time, label, position });
    }
    
    return markers;
  }, [totalDuration, zoom]);

  const handleZoom = useCallback((delta: number) => {
    setZoom(Math.max(MIN_PIXELS_PER_MS, Math.min(MAX_PIXELS_PER_MS, zoom + delta)));
  }, [zoom, setZoom]);


  useEffect(() => {
    const handleKeyPress = (e: KeyboardEvent) => {
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) {
        return;  
      }

      switch (e.key) {
        case "=":
        case "+":
          if (e.metaKey || e.ctrlKey) {
            e.preventDefault();
            handleZoom(0.1);
          }
          break;
        case "-":
          if (e.metaKey || e.ctrlKey) {
            e.preventDefault();
            handleZoom(-0.1);
          }
          break;
        case "0":
          if (e.metaKey || e.ctrlKey) {
            e.preventDefault();
            setZoom(1);
          }
          break;
        case "f":
          if (e.metaKey || e.ctrlKey) {
            e.preventDefault();
            setShowFilters(!showFilters);
          }
          break;
      }
    };

    window.addEventListener("keydown", handleKeyPress);
    return () => window.removeEventListener("keydown", handleKeyPress);
  }, [handleZoom, setZoom, showFilters, setShowFilters]);
 
  useEffect(() => {
    if (!isPlaying || playheadPosition === null || totalDuration === 0) return;

    const interval = setInterval(() => {
      setPlayheadPosition((prev) => {
        if (prev === null) return null;
        const next = prev + 50; // Advance 50ms per frame
        if (next >= totalDuration) {
          setIsPlaying(false);
          return totalDuration;
        }
        return next;
      });
    }, 50); // Update every 50ms

    return () => clearInterval(interval);
  }, [isPlaying, playheadPosition, totalDuration]);

  const toggleTrackCollapse = (trackId: string) => {
    setCollapsedTracks((prev) => {
      const next = new Set(prev);
      if (next.has(trackId)) {
        next.delete(trackId);
      } else {
        next.add(trackId);
      }
      return next;
    });
  };

  const toggleAllTracksCollapse = () => {
    if (collapsedTracks.size === 0) {
      setCollapsedTracks(new Set(filteredTraces.map(t => t.id)));
    } else {
      setCollapsedTracks(new Set());
    }
  };

  const allCollapsed = collapsedTracks.size > 0 && collapsedTracks.size === filteredTraces.length;

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <div>
            <CardTitle>Timeline View</CardTitle>
            <CardDescription>
              Visualize trace execution flow over time
              {hasActiveFilters && (
                <Badge variant="secondary" className="ml-2">
                  {formatNumber(filteredTraces.length)} of {formatNumber(traces.length)} traces
                </Badge>
              )}
            </CardDescription>
          </div>
          <div className="flex items-center gap-2">
            <div className="relative flex items-center gap-2">
              <Search className="absolute left-2 top-1/2 transform -translate-y-1/2 h-4 w-4 text-muted-foreground" />
              <Input
                type="text"
                placeholder="Search traces..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="pl-8 h-8 w-48 text-xs"
              />
              {traces.filter((t) => selectedTypes.has(t.entryType)).length > MAX_RENDERED_TRACES && (
                <span className="text-xs text-muted-foreground whitespace-nowrap">
                  Showing first {formatNumber(MAX_RENDERED_TRACES)} of {formatNumber(traces.filter((t) => selectedTypes.has(t.entryType)).length)}
                </span>
              )}
            </div>
            {filteredTraces.length > 0 && (
              <Button
                variant="outline"
                size="sm"
                onClick={toggleAllTracksCollapse}
                title={allCollapsed ? "Expand all tracks" : "Collapse all tracks"}
              >
                {allCollapsed ? (
                  <ChevronsUpDown className="h-4 w-4" />
                ) : (
                  <ChevronsDownUp className="h-4 w-4" />
                )}
              </Button>
            )}
            <Button
              variant={showFilters ? "default" : "outline"}
              size="sm"
              onClick={() => setShowFilters(!showFilters)}
              title="Toggle filters (⌘F)"
            >
              <Filter className="h-4 w-4 mr-2" />
              Filters
              {hasActiveFilters && (
                <Badge variant="secondary" className="ml-2 h-4 px-1 text-[10px]">
                  {selectedTypesArray.length < 4 ? "T" : ""}
                  {(startTime || endTime) ? "D" : ""}
                  {searchQuery ? "S" : ""}
                </Badge>
              )}
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={() => {
                if (!isPlaying) {
                  // Starting playback - reset to beginning if at end or not started
                  if (totalDuration > 0) {
                    if (playheadPosition === null || (playheadPosition !== null && playheadPosition >= totalDuration - 1)) {
                      setPlayheadPosition(0);
                    }
                    setIsPlaying(true);
                  }
                } else {
                  // Pausing
                  setIsPlaying(false);
                }
              }}
              disabled={totalDuration === 0}
              title="Animate execution flow - shows how traces execute over time"
            >
              {isPlaying ? <Pause className="h-4 w-4" /> : <Play className="h-4 w-4" />}
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={() => setShowMinimap(!showMinimap)}
              title="Toggle timeline overview"
            >
              {showMinimap ? <Minimize2 className="h-4 w-4" /> : <Maximize2 className="h-4 w-4" />}
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={() => handleZoom(-0.1)}
              disabled={zoom <= MIN_PIXELS_PER_MS}
              title="Zoom out (⌘-)"
            >
              <ZoomOut className="h-4 w-4" />
            </Button>
            <span className="text-xs text-muted-foreground min-w-[80px] text-center">
              {zoom.toFixed(2)}x
            </span>
            <Button
              variant="outline"
              size="sm"
              onClick={() => handleZoom(0.1)}
              disabled={zoom >= MAX_PIXELS_PER_MS}
              title="Zoom in (⌘+)"
            >
              <ZoomIn className="h-4 w-4" />
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={() => setZoom(1)}
              title="Reset zoom (⌘0)"
            >
              Reset
            </Button>
          </div>
        </div>

        {/* Filter Panel */}
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

            {/* Type Filters */}
            <div>
              <label className="text-xs font-medium text-muted-foreground mb-2 block">
                Trace Types
              </label>
              <div className="flex flex-wrap gap-2">
                {(["api", "event", "cron", "websocket"] as TraceType[]).map((type) => {
                  const Icon = entryTypeIcons[type];
                  const isSelected = selectedTypes.has(type);
                  return (
                    <Button
                      key={type}
                      variant={isSelected ? "default" : "outline"}
                      size="sm"
                      onClick={() => toggleType(type)}
                      className="h-8 text-xs"
                    >
                      {Icon && <Icon className="h-3 w-3 mr-1.5" />}
                      {type.charAt(0).toUpperCase() + type.slice(1)}
                    </Button>
                  );
                })}
              </div>
            </div>

            {/* Date/Time Range Filters */}
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
      <CardContent className="p-0 overflow-hidden">
        {filteredTraces.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-[600px] text-center p-8">
            <Filter className="h-12 w-12 text-muted-foreground mb-4" />
            <h3 className="text-lg font-semibold mb-2">No traces match the filters</h3>
            <p className="text-sm text-muted-foreground mb-4">
              {hasActiveFilters
                ? "Try adjusting your filter criteria or clear filters to see all traces."
                : "No traces available. Make API requests or trigger events to see traces."}
            </p>
            {hasActiveFilters && (
              <Button variant="outline" size="sm" onClick={clearFilters}>
                Clear Filters
              </Button>
            )}
          </div>
        ) : (
          <div className="relative flex h-[640px]">
            {/* Main Timeline Area */}
            <div className={cn(
              "relative transition-all duration-300 flex flex-col",
              selectedBlock ? "flex-1 min-w-0" : "w-full"
            )}>
            {/* Time Ruler */}
            <div className="sticky top-0 z-20 bg-background/95 backdrop-blur-sm border-b shadow-sm flex-shrink-0">
              <div className="relative h-10 overflow-hidden">
                <div
                  className="absolute top-0 h-full"
                  style={{ 
                    left: `${TRACK_LABEL_WIDTH}px`,
                    width: `${Math.max(timelineWidth, 1000)}px`, 
                    transform: `translateX(-${scrollLeft}px)` 
                  }}
                >
                  {timeMarkers.map((marker) => (
                    <div
                      key={marker.time}
                      className="absolute top-0 h-full border-l border-muted-foreground/40"
                      style={{ left: `${marker.position}px` }}
                    >
                      <div className="absolute top-0 left-0 h-3 w-px bg-foreground/60" />
                      <div className="absolute top-3 left-1 text-[10px] font-medium text-muted-foreground whitespace-nowrap bg-background/80 px-1 rounded">
                        {marker.label}
                      </div>
                    </div>
                  ))}
                  {/* Playhead on ruler */}
                  {playheadPosition !== null && (
                    <div
                      className="absolute top-0 h-full w-0.5 bg-red-500 z-10"
                      style={{ left: `${playheadPosition * zoom}px` }}
                    >
                      <div className="absolute -top-1 -left-1.5 w-3 h-3 bg-red-500 rounded-full border-2 border-white shadow" />
                    </div>
                  )}
                </div>
                {/* Scrubber area */}
                <div
                  className="absolute top-0 h-full cursor-pointer"
                  style={{
                    left: `${TRACK_LABEL_WIDTH}px`,
                    width: `${Math.max(timelineWidth, 1000)}px`,
                    transform: `translateX(-${scrollLeft}px)`,
                  }}
                  onMouseDown={(e) => {
                    const rect = e.currentTarget.getBoundingClientRect();
                    const x = e.clientX - rect.left + scrollLeft;
                    const time = x / zoom;
                    setPlayheadPosition(Math.max(0, Math.min(time, totalDuration)));
                    setIsPlaying(false);
                  }}
                />
              </div>
            </div>

          {/* Timeline Tracks */}
          <div
            ref={scrollAreaRef}
            className="flex-1 overflow-x-auto overflow-y-auto min-h-0"
            onScroll={(e) => {
              const target = e.target as HTMLElement;
              setScrollLeft(target.scrollLeft);
            }}
          >
            <div
              ref={timelineRef}
              className="relative"
              style={{ 
                width: `${TRACK_LABEL_WIDTH + Math.max(timelineWidth, 1000)}px`, 
                minHeight: `${timelineData.tracks.reduce((sum, t) => sum + (t.height || TRACK_HEIGHT), 0)}px` 
              }}
            >
              {/* Execution time indicator - shows current position in trace execution */}
              {playheadPosition !== null && (
                <div
                  className="absolute top-0 bottom-0 z-30 pointer-events-none"
                  style={{
                    left: `${TRACK_LABEL_WIDTH + playheadPosition * zoom}px`,
                    width: "2px",
                    backgroundColor: "#ef4444",
                    boxShadow: "0 0 4px rgba(239, 68, 68, 0.8)",
                  }}
                  title={`Execution time: ${formatDuration(playheadPosition)}`}
                >
                  <div className="absolute -top-2 -left-2 w-6 h-6 bg-red-500 rounded-full border-2 border-white shadow-lg flex items-center justify-center">
                    <div className="w-2 h-2 bg-white rounded-full" />
                  </div>
                </div>
              )}
              {/* Track Backgrounds */}
              {timelineData.tracks.map((track, index) => {
                const isCollapsed = track.isCollapsed || false;
                const trackHeight = track.height || TRACK_HEIGHT;
                return (
                  <div
                    key={track.id}
                    className={cn(
                      "absolute left-0 right-0 border-b border-border/50 transition-colors",
                      index % 2 === 0 ? "bg-background" : "bg-muted/20",
                      "hover:bg-muted/40"
                    )}
                    style={{
                      top: `${track.y}px`,
                      height: `${trackHeight}px`,
                    }}
                  >
                    {/* Track Label */}
                    <div 
                      className="sticky left-0 z-10 flex items-center h-full px-3 bg-background/95 backdrop-blur-sm border-r border-border shadow-sm"
                      style={{ width: `${TRACK_LABEL_WIDTH}px` }}
                    >
                      <Button
                        variant="ghost"
                        size="sm"
                        className="h-6 w-6 p-0 mr-1"
                        onClick={() => toggleTrackCollapse(track.id)}
                      >
                        {isCollapsed ? (
                          <ChevronRight className="h-4 w-4" />
                        ) : (
                          <ChevronDown className="h-4 w-4" />
                        )}
                      </Button>
                      <div className="flex items-center gap-2 flex-1 min-w-0">
                        {track.entryType && (() => {
                          const Icon = entryTypeIcons[track.entryType];
                          return Icon ? <Icon className="h-4 w-4 text-muted-foreground flex-shrink-0" /> : null;
                        })()}
                        <div className="flex-1 min-w-0">
                          <span className="text-sm font-medium truncate block">{track.label}</span>
                          {!isCollapsed && (
                            <div className="flex items-center gap-2 flex-wrap">
                            <span className="text-xs text-muted-foreground">
                              {formatDuration(track.duration)}
                            </span>
                              {track.metadata?.country && (
                                <Badge variant="outline" className="text-[10px] px-1.5 py-0">
                                  {track.metadata.country}
                                </Badge>
                              )}
                              {track.metadata?.ip_address || track.metadata?.ip || track.metadata?.ipAddress ? (
                                <span className="text-[10px] text-muted-foreground font-mono">
                                  {track.metadata.ip_address || track.metadata.ip || track.metadata.ipAddress}
                                </span>
                              ) : null}
                            </div>
                          )}
                        </div>
                        <Badge variant="outline" className="text-xs capitalize flex-shrink-0">
                          {track.entryType}
                        </Badge>
                      </div>
                    </div>
                  </div>
                );
              })}

              {/* Timeline Blocks */}
              {timelineData.blocks.map((block) => {
                // block.startTime is now relative (ms from trace start)
                const left = TRACK_LABEL_WIDTH + block.startTime * zoom;
                const width = Math.max(block.duration * zoom, 4);
                const track = timelineData.tracks.find((t) => t.id === block.traceId);
                const isCollapsed = track?.isCollapsed || false;
                const trackHeight = track?.height || TRACK_HEIGHT;
                const top = track ? track.y + (isCollapsed ? 2 : 8) : 0;
                const height = trackHeight - (isCollapsed ? 4 : 16);
                
                // Highlight slow operations (bottleneck detection)
                const isSlow = block.duration > 1000; // > 1s
                const isVerySlow = block.duration > 5000; // > 5s

                const isSelected = selectedBlock === block.id;
                const bgColor =
                  block.type === "trigger"
                    ? "bg-gradient-to-r from-blue-500 to-blue-600"
                    : block.type === "step"
                    ? block.status === "success"
                      ? "bg-gradient-to-r from-emerald-500 to-emerald-600"
                      : "bg-gradient-to-r from-red-500 to-red-600"
                    : block.entryType
                    ? block.entryType === "api"
                      ? "bg-gradient-to-r from-blue-500 to-blue-600"
                      : block.entryType === "event"
                      ? "bg-gradient-to-r from-yellow-500 to-yellow-600"
                      : block.entryType === "cron"
                      ? "bg-gradient-to-r from-purple-500 to-purple-600"
                      : "bg-gradient-to-r from-green-500 to-green-600"
                    : block.status === "success"
                    ? "bg-gradient-to-r from-emerald-500 to-emerald-600"
                    : block.status === "failed"
                    ? "bg-gradient-to-r from-red-500 to-red-600"
                    : "bg-gradient-to-r from-amber-500 to-amber-600";

                const trace = filteredTraces.find((t) => t.id === block.traceId);
                const step = block.stepIndex !== undefined && trace ? trace.steps[block.stepIndex] : null;
                const tooltipText = [
                  block.label,
                  `Duration: ${formatDuration(block.duration)}`,
                  isSlow ? "⚠️ Slow operation" : "",
                  step ? `Handler: ${step.handler_name}` : "",
                  trace ? `Trace: ${trace.id.slice(0, 8)}...` : "",
                  trace?.metadata?.datetime_utc ? `Time: ${new Date(trace.metadata.datetime_utc).toLocaleString('en-US', { timeZone: 'UTC', year: 'numeric', month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit', second: '2-digit', timeZoneName: 'short' })}` : "",
                  trace?.metadata?.ip_address || trace?.metadata?.ip || trace?.metadata?.ipAddress ? `IP: ${trace.metadata.ip_address || trace.metadata.ip || trace.metadata.ipAddress}` : "",
                  trace?.metadata?.country ? `Country: ${trace.metadata.country}` : "",
                  trace?.metadata?.location || trace?.metadata?.city ? `Location: ${trace.metadata.location || trace.metadata.city}${trace.metadata.region ? `, ${trace.metadata.region}` : ""}` : "",
                  trace?.metadata?.user_agent || trace?.metadata?.userAgent || trace?.metadata?.["user-agent"] ? `User Agent: ${(trace.metadata.user_agent || trace.metadata.userAgent || trace.metadata["user-agent"]).substring(0, 50)}${(trace.metadata.user_agent || trace.metadata.userAgent || trace.metadata["user-agent"]).length > 50 ? "..." : ""}` : "",
                ].filter(Boolean).join("\n");

                return (
                  <div
                    key={block.id}
                    className={cn(
                      "absolute rounded-md border-2 cursor-pointer",
                      "hover:shadow-xl hover:z-10",
                      bgColor,
                      isSelected
                        ? "ring-2 ring-primary ring-offset-2 z-20 shadow-lg"
                        : "border-white/30 shadow-md",
                      block.type === "trigger" && "opacity-90",
                      isVerySlow && "ring-2 ring-orange-500 ring-offset-1",
                      isSlow && !isVerySlow && "ring-1 ring-yellow-500"
                    )}
                    style={{
                      left: `${left}px`,
                      width: `${width}px`,
                      top: `${top}px`,
                      height: `${height}px`,
                    }}
                    onClick={(e) => {
                      e.stopPropagation();
                      setSelectedBlock(block.id);
                    }}
                    title={tooltipText}
                  >
                    <div className="h-full flex items-center px-2 overflow-hidden relative">
                      <div className="absolute inset-0 bg-white/10 opacity-0 hover:opacity-100 transition-opacity rounded-md" />
                      <div className="flex-1 min-w-0 relative z-10">
                        <div className="text-xs font-semibold text-white truncate drop-shadow-sm">
                          {block.label}
                        </div>
                        {width > 80 && !isCollapsed && (
                          <div className="text-[10px] text-white/90 drop-shadow-sm">
                            {formatDuration(block.duration)}
                          </div>
                        )}
                      </div>
                      {block.type === "trigger" && width > 40 && (
                        <Zap className="h-3 w-3 text-white/90 flex-shrink-0 ml-1 drop-shadow-sm" />
                      )}
                    </div>
                  </div>
                );
              })}
            </div>
          </div>
          </div>

            {/* Right Drawer for Block Details */}
        {selectedBlock && (
              <div className={cn(
                "w-96 border-l bg-background shadow-2xl flex flex-col flex-shrink-0",
                "h-[640px] overflow-hidden"
              )}>
                <div className="border-b p-4 flex items-center justify-between flex-shrink-0 bg-background">
                  <h3 className="font-semibold text-lg">Block Details</h3>
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-8 w-8"
                    onClick={(e) => {
                      e.stopPropagation();
                      setSelectedBlock(null);
                    }}
                  >
                    <X className="h-4 w-4" />
                  </Button>
                </div>
                <ScrollArea className="flex-1 overflow-hidden">
                  <div className="p-4">
            {(() => {
              const block = timelineData.blocks.find((b) => b.id === selectedBlock);
              if (!block) return null;

              const trace = filteredTraces.find((t) => t.id === block.traceId);
              const step = block.stepIndex !== undefined && trace ? trace.steps[block.stepIndex] : null;
              const trigger =
                block.triggerIndex !== undefined &&
                step &&
                step.triggered_events
                  ? step.triggered_events[block.triggerIndex]
                  : null;

              return (
                        <div className="space-y-4">
                  <div className="flex items-center justify-between">
                            <h4 className="font-semibold text-base">{block.label}</h4>
                    <Badge variant="outline">{block.type}</Badge>
                  </div>
                          
                          <div className="space-y-3">
                    <div>
                              <span className="text-sm font-medium text-muted-foreground">Duration</span>
                              <div className="mt-1 flex items-center gap-2">
                                <span className="text-base font-semibold">{formatDuration(block.duration)}</span>
                      {block.duration > 1000 && (
                                  <Badge variant="destructive" className="text-xs">
                          Slow
                        </Badge>
                      )}
                                {block.duration > 5000 && (
                                  <Badge variant="destructive" className="text-xs">
                                    Very Slow
                                  </Badge>
                                )}
                    </div>
                            </div>

                    <div>
                              <span className="text-sm font-medium text-muted-foreground">Start Time</span>
                              <div className="mt-1 text-base">
                      {formatDuration(block.startTime)} from trace start
                    </div>
                            </div>

                    {step && (
                      <>
                                <div className="border-t pt-3">
                                  <span className="text-sm font-medium text-muted-foreground">Handler</span>
                                  <div className="mt-1 text-base font-mono">{step.handler_name}</div>
                        </div>
                        <div>
                                  <span className="text-sm font-medium text-muted-foreground">Status</span>
                                  <div className="mt-1">
                          <Badge variant={step.success ? "default" : "destructive"} className="text-xs">
                            {step.success ? "Success" : "Failed"}
                          </Badge>
                        </div>
                                </div>
                                {step.path && (
                                  <div>
                                    <span className="text-sm font-medium text-muted-foreground">Path</span>
                                    <div className="mt-1 text-sm font-mono text-muted-foreground">{step.path}</div>
                                  </div>
                                )}
                                {step.error && (
                                  <div className="border-t pt-3">
                                    <span className="text-sm font-medium text-muted-foreground">Error</span>
                                    <div className="mt-1 text-sm text-destructive font-mono bg-destructive/10 p-2 rounded">
                                      {step.error}
                                    </div>
                                  </div>
                                )}
                      </>
                    )}

                    {trigger && trace && (
                      <>
                                <div className="border-t pt-3">
                                  <span className="text-sm font-medium text-muted-foreground">Event Name</span>
                                  <div className="mt-1 text-base font-semibold">{trigger.event_name}</div>
                        </div>
                        <div>
                                  <span className="text-sm font-medium text-muted-foreground">Triggered At</span>
                                  <div className="mt-1 text-base">
                          {formatDuration(
                            new Date(trigger.timestamp).getTime() - new Date(trace.startedAt).getTime()
                          )}{" "}
                          from trace start
                                  </div>
                                </div>
                                <div>
                                  <span className="text-sm font-medium text-muted-foreground">Event Duration</span>
                                  <div className="mt-1 text-base">{formatDuration(trigger.duration_ms)}</div>
                        </div>
                      </>
                    )}

                            {trace && (
                              <>
                                {trace.metadata?.datetime_utc && (
                                  <div className="border-t pt-3">
                                    <span className="text-sm font-medium text-muted-foreground">Date & Time (UTC)</span>
                                    <div className="mt-1 text-sm text-foreground">
                                      {new Date(trace.metadata.datetime_utc).toLocaleString('en-US', {
                                        timeZone: 'UTC',
                                        year: 'numeric',
                                        month: 'long',
                                        day: 'numeric',
                                        hour: '2-digit',
                                        minute: '2-digit',
                                        second: '2-digit',
                                        timeZoneName: 'short'
                                      })}
                                    </div>
                                    <div className="mt-1 text-xs font-mono text-muted-foreground">
                                      {trace.metadata.datetime_utc}
                                    </div>
                                  </div>
                                )}
                                {(trace.metadata?.ip_address || trace.metadata?.ip || trace.metadata?.ipAddress) && (
                                  <div>
                                    <span className="text-sm font-medium text-muted-foreground">IP Address</span>
                                    <div className="mt-1 text-sm font-mono text-foreground">
                                      {trace.metadata.ip_address || trace.metadata.ip || trace.metadata.ipAddress}
                                    </div>
                                  </div>
                                )}
                                {(trace.metadata?.country || trace.metadata?.country_code) && (
                                  <div>
                                    <span className="text-sm font-medium text-muted-foreground">Country</span>
                                    <div className="mt-1 text-sm text-foreground">
                                      {trace.metadata.country || trace.metadata.country_code}
                                    </div>
                                  </div>
                                )}
                                {(trace.metadata?.location || trace.metadata?.city) && (
                                  <div>
                                    <span className="text-sm font-medium text-muted-foreground">Location</span>
                                    <div className="mt-1 text-sm text-foreground">
                                      {trace.metadata.location || trace.metadata.city}
                                      {trace.metadata.city && trace.metadata.region && `, ${trace.metadata.region}`}
                                    </div>
                                  </div>
                                )}
                                {(trace.metadata?.user_agent || trace.metadata?.userAgent || trace.metadata?.["user-agent"]) && (
                                  <div>
                                    <span className="text-sm font-medium text-muted-foreground">User Agent</span>
                                    <div className="mt-1 text-xs font-mono text-muted-foreground break-all">
                                      {trace.metadata.user_agent || trace.metadata.userAgent || trace.metadata["user-agent"]}
                                    </div>
                                  </div>
                                )}
                                <div className="border-t pt-3">
                                  <span className="text-sm font-medium text-muted-foreground">Trace ID</span>
                                  <div className="mt-1 text-xs font-mono text-muted-foreground break-all">
                                    {trace.id}
                                  </div>
                                </div>
                              </>
                            )}
                  </div>
                </div>
              );
            })()}
                  </div>
                </ScrollArea>
              </div>
            )}
          </div>
        )}
      </CardContent>
    </Card>
  );
}


