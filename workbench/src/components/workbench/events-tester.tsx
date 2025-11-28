"use client";

import { useState, useEffect } from "react";
import { Send, Loader2, CheckCircle2, XCircle, Copy, Zap } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import type { EventEndpoint } from "@/lib/workbench-data";
import { fetchTypeSchema } from "@/lib/workbench-data";
import { api, ApiError } from "@/lib/api";
import { JsonEditor } from "@/components/ui/json-editor";

type TriggerState = {
  loading: boolean;
  success: boolean;
  error: string | null;
  response: string | null;
};

export function EventsTester({ events }: { events: EventEndpoint[] }) {
  const [selectedEvent, setSelectedEvent] = useState<EventEndpoint | null>(events[0] || null);
  const [payload, setPayload] = useState<string>("");
  const [payloadSchema, setPayloadSchema] = useState<unknown | null>(null);
  const [triggerState, setTriggerState] = useState<TriggerState>({
    loading: false,
    success: false,
    error: null,
    response: null,
  });

  useEffect(() => {
    if (selectedEvent?.payload) {
      fetchTypeSchema(selectedEvent.payload).then(setPayloadSchema);
    } else {
      // eslint-disable-next-line react-hooks/exhaustive-deps
      setPayloadSchema(null);
    }
  }, [selectedEvent?.payload]);

  const handleTrigger = async () => {
    if (!selectedEvent) return;

    setTriggerState({ loading: true, success: false, error: null, response: null });

    try {
      let payloadData: unknown = {};
      if (payload.trim()) {
        try {
          payloadData = JSON.parse(payload);
        } catch {
          // If not valid JSON, treat as string
          payloadData = payload;
        }
      } else if (selectedEvent.payload) {
        // Try to parse the default payload from schema
        try {
          payloadData = JSON.parse(selectedEvent.payload);
        } catch {
          payloadData = selectedEvent.payload;
        }
      }

      const data = await api.post<{
        success: boolean;
        event: string;
        payload: unknown;
      }>(`/api/workbench/events/${selectedEvent.name}/trigger`, {
        payload: payloadData,
      });

      setTriggerState({
        loading: false,
        success: true,
        error: null,
        response: JSON.stringify(data, null, 2),
      });
    } catch (error) {
      const errorMessage =
        error instanceof ApiError
          ? error.message
          : error instanceof Error
            ? error.message
            : "Unknown error";

      setTriggerState({
        loading: false,
        success: false,
        error: errorMessage,
        response: null,
      });
    }
  };

  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text);
  };

  const loadDefaultPayload = () => {
    if (selectedEvent?.payload) {
      try {
        // Try to format as JSON if it's valid JSON
        const parsed = JSON.parse(selectedEvent.payload);
        setPayload(JSON.stringify(parsed, null, 2));
      } catch {
        // If not JSON, use as-is
        setPayload(selectedEvent.payload);
      }
    } else {
      setPayload("{}");
    }
  };

  return (
    <div className="space-y-4">
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
        <Card className="lg:col-span-1">
          <CardHeader>
            <CardTitle>Events</CardTitle>
            <CardDescription>Select an event to trigger</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="space-y-2 max-h-[600px] overflow-y-auto">
              {events.length === 0 ? (
                <p className="text-sm text-muted-foreground">No events found</p>
              ) : (
                events.map((event) => (
                  <button
                    key={event.name}
                    onClick={() => {
                      setSelectedEvent(event);
                      setPayload("");
                      setTriggerState({ loading: false, success: false, error: null, response: null });
                    }}
                    className={cn(
                      "w-full text-left p-3 rounded-md border transition-colors",
                      selectedEvent?.name === event.name
                        ? "bg-accent border-accent-foreground/20"
                        : "border-border hover:bg-accent/50"
                    )}
                  >
                    <div className="flex items-center gap-2 mb-1">
                      <Zap className="h-4 w-4 text-yellow-500" />
                      <span className="text-sm font-medium">{event.name}</span>
                    </div>
                    {event.handlers.length > 0 && (
                      <div className="flex flex-wrap gap-1 mt-1">
                        {event.handlers.slice(0, 2).map((handler, idx) => (
                          <Badge key={idx} variant="secondary" className="text-xs">
                            {handler}
                          </Badge>
                        ))}
                        {event.handlers.length > 2 && (
                          <Badge variant="secondary" className="text-xs">
                            +{event.handlers.length - 2}
                          </Badge>
                        )}
                      </div>
                    )}
                  </button>
                ))
              )}
            </div>
          </CardContent>
        </Card>

        <Card className="lg:col-span-2">
          <CardHeader>
            <CardTitle>Trigger Event</CardTitle>
            <CardDescription>
              {selectedEvent
                ? `Trigger the "${selectedEvent.name}" event with a custom payload`
                : "Select an event to trigger"}
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            {selectedEvent && (
              <>
                <div className="space-y-2">
                  <div className="flex items-center justify-between">
                    <label className="text-sm font-medium">Event Payload (JSON)</label>
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={loadDefaultPayload}
                      className="text-xs"
                    >
                      Load Default
                    </Button>
                  </div>
                  <div className="rounded-md border border-input overflow-hidden">
                    <JsonEditor
                      value={payload}
                      onChange={setPayload}
                      placeholder={selectedEvent.payload || '{\n  "key": "value"\n}'}
                      height="300px"
                      className="w-full"
                      jsonSchema={payloadSchema || undefined}
                    />
                  </div>
                </div>

                <div className="flex items-center gap-2">
                  <Button
                    onClick={handleTrigger}
                    disabled={triggerState.loading}
                    className="flex-1"
                  >
                    {triggerState.loading ? (
                      <>
                        <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                        Triggering...
                      </>
                    ) : (
                      <>
                        <Send className="h-4 w-4 mr-2" />
                        Trigger Event
                      </>
                    )}
                  </Button>
                </div>

                {selectedEvent.handlers.length > 0 && (
                  <div className="space-y-1">
                    <label className="text-sm font-medium">Handlers</label>
                    <div className="flex flex-wrap gap-2">
                      {selectedEvent.handlers.map((handler, idx) => (
                        <Badge key={idx} variant="outline" className="text-xs">
                          {handler}
                        </Badge>
                      ))}
                    </div>
                  </div>
                )}

                {selectedEvent.triggers.length > 0 && (
                  <div className="space-y-1">
                    <label className="text-sm font-medium">Triggers</label>
                    <div className="flex flex-wrap gap-2">
                      {selectedEvent.triggers.map((trigger, idx) => (
                        <Badge key={idx} variant="secondary" className="text-xs">
                          {trigger}
                        </Badge>
                      ))}
                    </div>
                  </div>
                )}

                {triggerState.error && (
                  <div className="space-y-2">
                    <div className="flex items-center gap-2 text-red-500">
                      <XCircle className="h-4 w-4" />
                      <span className="text-sm font-medium">Error</span>
                    </div>
                    <p className="text-sm text-red-500">{triggerState.error}</p>
                  </div>
                )}

                {triggerState.success && triggerState.response && (
                  <div className="space-y-2">
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-2 text-green-500">
                        <CheckCircle2 className="h-4 w-4" />
                        <span className="text-sm font-medium">Event Triggered Successfully</span>
                      </div>
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => copyToClipboard(triggerState.response || "")}
                      >
                        <Copy className="h-4 w-4" />
                      </Button>
                    </div>
                    <pre className="w-full min-h-[100px] max-h-[300px] overflow-auto rounded-md border border-input bg-muted/50 p-3 text-xs font-mono">
                      {triggerState.response}
                    </pre>
                  </div>
                )}
              </>
            )}

            {!selectedEvent && (
              <p className="text-sm text-muted-foreground">
                Select an event from the list to trigger it.
              </p>
            )}
          </CardContent>
        </Card>
      </div>
    </div>
  );
}

