"use client";

import { useState, useEffect } from "react";
import { Send, Loader2, CheckCircle2, XCircle, Copy, Clock } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { cn } from "@/lib/utils";
import type { ApiEndpoint, WebSocketEndpoint } from "@/lib/workbench-data";
import { fetchTypeSchema } from "@/lib/workbench-data";
import { getApiBaseUrl } from "@/lib/api";
import { JsonEditor } from "@/components/ui/json-editor";

type RequestMethod = "GET" | "POST" | "PUT" | "PATCH" | "DELETE";

type RequestState = {
  method: RequestMethod;
  url: string;
  headers: Record<string, string>;
  body: string;
};

type ResponseState = {
  status: number | null;
  statusText: string;
  headers: Record<string, string>;
  body: string;
  duration: number | null;
  timestamp: string | null;
};

type WebSocketState = {
  connected: boolean;
  messages: Array<{ type: "sent" | "received"; data: string; timestamp: string }>;
  connectionId: string | null;
};

export function ApiTester({ apis, websockets }: { apis: ApiEndpoint[]; websockets: WebSocketEndpoint[] }) {
  const [selectedApi, setSelectedApi] = useState<ApiEndpoint | null>(apis[0] || null);
  const [selectedWs, setSelectedWs] = useState<WebSocketEndpoint | null>(websockets[0] || null);
  const [request, setRequest] = useState<RequestState>({
    method: "GET",
    url: "",
    headers: { "Content-Type": "application/json" },
    body: "",
  });
  const [response, setResponse] = useState<ResponseState>({
    status: null,
    statusText: "",
    headers: {},
    body: "",
    duration: null,
    timestamp: null,
  });
  const [wsState, setWsState] = useState<WebSocketState>({
    connected: false,
    messages: [],
    connectionId: null,
  });
  const [wsSocket, setWsSocket] = useState<WebSocket | null>(null);
  const [wsMessage, setWsMessage] = useState("");
  const [loading, setLoading] = useState(false);
  const [bodySchema, setBodySchema] = useState<unknown | null>(null);
  const [wsMessageSchema, setWsMessageSchema] = useState<unknown | null>(null);

  const baseUrl = getApiBaseUrl();

  // Fetch JSON schema for the body type
  useEffect(() => {
    if (selectedApi?.body) {
      fetchTypeSchema(selectedApi.body).then(setBodySchema);
    } else {
      setBodySchema(null);
    }
  }, [selectedApi?.body]);

  // Fetch JSON schema for the WebSocket message type
  useEffect(() => {
    if (selectedWs?.message) {
      fetchTypeSchema(selectedWs.message).then(setWsMessageSchema);
    } else {
      setWsMessageSchema(null);
    }
  }, [selectedWs?.message]);

  useEffect(() => {
    if (selectedApi) {
      setRequest({
        method: selectedApi.method as RequestMethod,
        url: `${baseUrl}${selectedApi.path}`,
        headers: { "Content-Type": "application/json" },
        body: selectedApi.body || "",
      });
      setResponse({
        status: null,
        statusText: "",
        headers: {},
        body: "",
        duration: null,
        timestamp: null,
      });
    }
  }, [selectedApi, baseUrl]);

  const sendRequest = async () => {
    if (!request.url) return;

    setLoading(true);
    const startTime = performance.now();

    try {
      const fetchOptions: RequestInit = {
        method: request.method,
        headers: request.headers,
      };

      if (request.method !== "GET" && request.body) {
        fetchOptions.body = request.body;
      }

      const res = await fetch(request.url, fetchOptions);
      const duration = performance.now() - startTime;

      const responseHeaders: Record<string, string> = {};
      res.headers.forEach((value, key) => {
        responseHeaders[key] = value;
      });

      const body = await res.text();
      let formattedBody = body;
      try {
        const json = JSON.parse(body);
        formattedBody = JSON.stringify(json, null, 2);
      } catch {
        // Not JSON, use as-is
      }

      setResponse({
        status: res.status,
        statusText: res.statusText,
        headers: responseHeaders,
        body: formattedBody,
        duration: Math.round(duration),
        timestamp: new Date().toISOString(),
      });
    } catch (error) {
      const duration = performance.now() - startTime;
      setResponse({
        status: null,
        statusText: "Error",
        headers: {},
        body: error instanceof Error ? error.message : "Unknown error",
        duration: Math.round(duration),
        timestamp: new Date().toISOString(),
      });
    } finally {
      setLoading(false);
    }
  };

  const connectWebSocket = () => {
    if (!selectedWs || wsState.connected) return;

    try {
      const wsUrl = baseUrl.replace(/^https?/, (match) => match === "https" ? "wss" : "ws") + selectedWs.path;
      const socket = new WebSocket(wsUrl);

      socket.onopen = () => {
        setWsState({
          connected: true,
          messages: [
            {
              type: "received",
              data: "Connected to WebSocket",
              timestamp: new Date().toISOString(),
            },
          ],
          connectionId: null,
        });
        setWsSocket(socket);
      };

      socket.onmessage = (event) => {
        setWsState((prev) => ({
          ...prev,
          messages: [
            ...prev.messages,
            {
              type: "received",
              data: event.data,
              timestamp: new Date().toISOString(),
            },
          ],
        }));
      };

      socket.onerror = (error) => {
        setWsState((prev) => ({
          ...prev,
          messages: [
            ...prev.messages,
            {
              type: "received",
              data: `Error: ${error}`,
              timestamp: new Date().toISOString(),
            },
          ],
        }));
      };

      socket.onclose = () => {
        setWsState({
          connected: false,
          messages: [],
          connectionId: null,
        });
        setWsSocket(null);
      };
    } catch (error) {
      setWsState((prev) => ({
        ...prev,
        messages: [
          ...prev.messages,
          {
            type: "received",
            data: `Connection error: ${error instanceof Error ? error.message : "Unknown error"}`,
            timestamp: new Date().toISOString(),
          },
        ],
      }));
    }
  };

  const disconnectWebSocket = () => {
    if (wsSocket) {
      wsSocket.close();
      setWsSocket(null);
    }
  };

  const sendWebSocketMessage = () => {
    if (!wsSocket || !wsState.connected || !wsMessage) return;

    // Try to parse as JSON, if it fails send as string
    let messageToSend: string;
    try {
      const parsed = JSON.parse(wsMessage);
      messageToSend = JSON.stringify(parsed);
    } catch {
      messageToSend = wsMessage;
    }

    wsSocket.send(messageToSend);
    setWsState((prev) => ({
      ...prev,
      messages: [
        ...prev.messages,
        {
          type: "sent",
          data: messageToSend,
          timestamp: new Date().toISOString(),
        },
      ],
    }));
    setWsMessage("");
  };

  const loadMessageExample = () => {
    if (!wsMessageSchema || typeof wsMessageSchema !== "object") {
      return;
    }

    const schema = wsMessageSchema as { properties?: Record<string, unknown>; required?: string[] };
    const example: Record<string, unknown> = {};

    if (schema.properties) {
      Object.entries(schema.properties).forEach(([key, prop]) => {
        const propSchema = prop as { type?: string; format?: string };
        if (propSchema.type === "string") {
          example[key] = propSchema.format === "date-time" ? new Date().toISOString() : `example_${key}`;
        } else if (propSchema.type === "number") {
          example[key] = 0;
        } else if (propSchema.type === "boolean") {
          example[key] = false;
        } else if (propSchema.type === "array") {
          example[key] = [];
        } else if (propSchema.type === "object") {
          example[key] = {};
        } else {
          example[key] = null;
        }
      });
    }

    setWsMessage(JSON.stringify(example, null, 2));
  };

  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text);
  };

  return (
    <div className="space-y-6">
      <Tabs defaultValue="api" className="w-full">
        <TabsList className="grid w-full grid-cols-2">
          <TabsTrigger value="api">API Testing</TabsTrigger>
          <TabsTrigger value="websocket">WebSocket Testing</TabsTrigger>
        </TabsList>

        <TabsContent value="api" className="space-y-4">
          <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
            <Card className="lg:col-span-1">
              <CardHeader>
                <CardTitle>Endpoints</CardTitle>
                <CardDescription>Select an API endpoint to test</CardDescription>
              </CardHeader>
              <CardContent>
                <div className="space-y-2 max-h-[600px] overflow-y-auto">
                  {apis.length === 0 ? (
                    <p className="text-sm text-muted-foreground">No API endpoints found</p>
                  ) : (
                    apis.map((api) => (
                      <button
                        key={api.name}
                        onClick={() => setSelectedApi(api)}
                        className={cn(
                          "w-full text-left p-3 rounded-md border transition-colors",
                          selectedApi?.name === api.name
                            ? "bg-accent border-accent-foreground/20"
                            : "border-border hover:bg-accent/50"
                        )}
                      >
                        <div className="flex items-center gap-2 mb-1">
                          <Badge variant="outline" className="text-xs">
                            {api.method}
                          </Badge>
                          <span className="text-sm font-medium">{api.name}</span>
                        </div>
                        <p className="text-xs text-muted-foreground truncate">{api.path}</p>
                      </button>
                    ))
                  )}
                </div>
              </CardContent>
            </Card>

            <Card className="lg:col-span-2">
              <CardHeader>
                <CardTitle>Request</CardTitle>
                <CardDescription>Configure and send your API request</CardDescription>
              </CardHeader>
              <CardContent className="space-y-4">
                <div className="flex gap-2">
                  <select
                    value={request.method}
                    onChange={(e) => setRequest({ ...request, method: e.target.value as RequestMethod })}
                    className="h-9 rounded-md border border-input bg-transparent px-3 text-sm"
                  >
                    <option value="GET">GET</option>
                    <option value="POST">POST</option>
                    <option value="PUT">PUT</option>
                    <option value="PATCH">PATCH</option>
                    <option value="DELETE">DELETE</option>
                  </select>
                  <Input
                    value={request.url}
                    onChange={(e) => setRequest({ ...request, url: e.target.value })}
                    placeholder="Enter URL"
                    className="flex-1"
                  />
                  <Button onClick={sendRequest} disabled={loading || !request.url}>
                    {loading ? (
                      <Loader2 className="h-4 w-4 animate-spin" />
                    ) : (
                      <Send className="h-4 w-4" />
                    )}
                  </Button>
                </div>

                <div>
                  <label className="text-sm font-medium mb-2 block">Request Body</label>
                  <div className="rounded-md border border-input overflow-hidden">
                    <JsonEditor
                      value={request.body}
                      onChange={(value) => setRequest({ ...request, body: value })}
                      placeholder={selectedApi?.body ? `Example: ${selectedApi.body}` : '{\n  "key": "value"\n}'}
                      height="300px"
                      className="w-full"
                      jsonSchema={bodySchema || undefined}
                    />
                  </div>
                </div>

                {response.status !== null && (
                  <div className="space-y-2">
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-2">
                        {response.status >= 200 && response.status < 300 ? (
                          <CheckCircle2 className="h-4 w-4 text-green-500" />
                        ) : (
                          <XCircle className="h-4 w-4 text-red-500" />
                        )}
                        <span className="text-sm font-medium">
                          {response.status} {response.statusText}
                        </span>
                        {response.duration && (
                          <span className="text-xs text-muted-foreground flex items-center gap-1">
                            <Clock className="h-3 w-3" />
                            {response.duration}ms
                          </span>
                        )}
                      </div>
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => copyToClipboard(response.body)}
                      >
                        <Copy className="h-4 w-4" />
                      </Button>
                    </div>
                    <div>
                      <label className="text-sm font-medium mb-2 block">Response Body</label>
                      <pre className="w-full min-h-[200px] max-h-[400px] overflow-auto rounded-md border border-input bg-muted/50 p-3 text-xs font-mono">
                        {response.body}
                      </pre>
                    </div>
                  </div>
                )}
              </CardContent>
            </Card>
          </div>
        </TabsContent>

        <TabsContent value="websocket" className="space-y-4">
          <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
            <Card className="lg:col-span-1">
              <CardHeader>
                <CardTitle>WebSockets</CardTitle>
                <CardDescription>Select a WebSocket endpoint</CardDescription>
              </CardHeader>
              <CardContent>
                <div className="space-y-2 max-h-[600px] overflow-y-auto">
                  {websockets.length === 0 ? (
                    <p className="text-sm text-muted-foreground">No WebSocket endpoints found</p>
                  ) : (
                    websockets.map((ws) => (
                      <button
                        key={ws.name}
                        onClick={() => setSelectedWs(ws)}
                        className={cn(
                          "w-full text-left p-3 rounded-md border transition-colors",
                          selectedWs?.name === ws.name
                            ? "bg-accent border-accent-foreground/20"
                            : "border-border hover:bg-accent/50"
                        )}
                      >
                        <div className="flex items-center gap-2 mb-1">
                          <Badge variant="outline" className="text-xs">
                            WS
                          </Badge>
                          <span className="text-sm font-medium">{ws.name}</span>
                        </div>
                        <p className="text-xs text-muted-foreground truncate">{ws.path}</p>
                      </button>
                    ))
                  )}
                </div>
              </CardContent>
            </Card>

            <Card className="lg:col-span-2">
              <CardHeader>
                <CardTitle>WebSocket Connection</CardTitle>
                <CardDescription>Connect and test WebSocket endpoints</CardDescription>
              </CardHeader>
              <CardContent className="space-y-4">
                <div className="flex gap-2">
                  <Input
                    value={selectedWs ? `${baseUrl.replace(/^https?/, (match) => match === "https" ? "wss" : "ws")}${selectedWs.path}` : ""}
                    placeholder="WebSocket URL"
                    readOnly
                    className="flex-1"
                  />
                  {!wsState.connected ? (
                    <Button onClick={connectWebSocket} disabled={!selectedWs}>
                      Connect
                    </Button>
                  ) : (
                    <Button onClick={disconnectWebSocket} variant="destructive">
                      Disconnect
                    </Button>
                  )}
                </div>

                {wsState.connected && (
                  <div className="space-y-2">
                    <div className="flex items-center gap-2">
                      <div className="h-2 w-2 rounded-full bg-green-500" />
                      <span className="text-sm text-muted-foreground">Connected</span>
                    </div>
                    <div className="space-y-2">
                      <div className="flex items-center justify-between">
                        <label className="text-sm font-medium">Message</label>
                        {selectedWs?.message && (
                          <Button
                            variant="ghost"
                            size="sm"
                            onClick={loadMessageExample}
                            className="text-xs"
                          >
                            Load Example
                          </Button>
                        )}
                      </div>
                      <div className="rounded-md border border-input overflow-hidden">
                        <JsonEditor
                          value={wsMessage}
                          onChange={setWsMessage}
                          placeholder={selectedWs?.message ? `Example: ${selectedWs.message}` : 'Enter message to send'}
                          height="200px"
                          className="w-full"
                          jsonSchema={wsMessageSchema || undefined}
                        />
                      </div>
                      <Button onClick={sendWebSocketMessage} disabled={!wsMessage.trim()} className="w-full">
                        <Send className="h-4 w-4 mr-2" />
                        Send Message
                      </Button>
                    </div>
                  </div>
                )}

                <div>
                  <label className="text-sm font-medium mb-2 block">Messages</label>
                  <div className="w-full min-h-[300px] max-h-[500px] overflow-auto rounded-md border border-input bg-muted/50 p-3 space-y-2">
                    {wsState.messages.length === 0 ? (
                      <p className="text-sm text-muted-foreground">No messages yet</p>
                    ) : (
                      wsState.messages.map((msg, idx) => (
                        <div
                          key={idx}
                          className={cn(
                            "p-2 rounded text-xs font-mono",
                            msg.type === "sent"
                              ? "bg-blue-500/10 text-blue-500"
                              : "bg-green-500/10 text-green-500"
                          )}
                        >
                          <div className="flex items-center justify-between mb-1">
                            <span className="font-semibold">
                              {msg.type === "sent" ? "→ Sent" : "← Received"}
                            </span>
                            <span className="text-muted-foreground">
                              {new Date(msg.timestamp).toLocaleTimeString()}
                            </span>
                          </div>
                          <pre className="whitespace-pre-wrap break-words">{msg.data}</pre>
                        </div>
                      ))
                    )}
                  </div>
                </div>
              </CardContent>
            </Card>
          </div>
        </TabsContent>
      </Tabs>
    </div>
  );
}

