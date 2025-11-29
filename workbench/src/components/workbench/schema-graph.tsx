"use client";

import "@xyflow/react/dist/style.css";

import {
  Background,
  Controls,
  ReactFlow,
  ReactFlowProvider,
  useEdgesState,
  useNodesState,
  Handle,
  Position,
  NodeProps,
} from "@xyflow/react";
import { useEffect, useMemo, useState } from "react";
import dagre from "dagre";
import { Prism as SyntaxHighlighter } from "react-syntax-highlighter";
import { vscDarkPlus } from "react-syntax-highlighter/dist/esm/styles/prism";
import { Copy, Check, FileCode, Code, Database } from "lucide-react";
import { Panel, PanelGroup, PanelResizeHandle } from "react-resizable-panels";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";
import type {
  SchemaGraph as SchemaGraphData,
  SchemaGraphNode,
} from "@/lib/workbench-data";

type SchemaGraphProps = SchemaGraphData;

const VIEW_OPTIONS = [
  { value: "all", label: "All nodes" },
  { value: "schema", label: "Schemas" },
  { value: "handler", label: "Handlers" },
] as const;

export function SchemaGraph({
  nodes: schemaNodes,
  edges: schemaEdges,
  root,
}: SchemaGraphProps) {
  const [search, setSearch] = useState("");
  const [view, setView] =
    useState<(typeof VIEW_OPTIONS)[number]["value"]>("all");
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);

  const filteredNodes = useMemo(() => {
    return schemaNodes.filter((node) => {
      if (view !== "all" && node.type !== view) return false;
      if (!search) return true;
      const query = search.toLowerCase();
      return (
        node.label.toLowerCase().includes(query) ||
        node.bucket.toLowerCase().includes(query) ||
        node.path.toLowerCase().includes(query)
      );
    });
  }, [schemaNodes, view, search]);

  const filteredEdges = useMemo(() => {
    const allowed = new Set(filteredNodes.map((node) => node.id));
    return schemaEdges.filter(
      (edge) => allowed.has(edge.source) && allowed.has(edge.target)
    );
  }, [schemaEdges, filteredNodes]);

  const effectiveSelectedNodeId = useMemo(() => {
    if (
      selectedNodeId &&
      filteredNodes.some((node) => node.id === selectedNodeId)
    ) {
      return selectedNodeId;
    }
    return null;
  }, [selectedNodeId, filteredNodes]);

  const selectedNode =
    schemaNodes.find(
      (node) => node.id === (effectiveSelectedNodeId ?? selectedNodeId)
    ) ?? null;

  const highlight = useMemo(() => {
    if (!effectiveSelectedNodeId) return null;
    const nodeSet = new Set<string>([effectiveSelectedNodeId]);
    const edgeSet = new Set<string>();
    filteredEdges.forEach((edge) => {
      if (edge.source === effectiveSelectedNodeId) {
        nodeSet.add(edge.target);
        edgeSet.add(edge.id);
      } else if (edge.target === effectiveSelectedNodeId) {
        nodeSet.add(edge.source);
        edgeSet.add(edge.id);
      }
    });
    return { nodes: nodeSet, edges: edgeSet };
  }, [effectiveSelectedNodeId, filteredEdges]);

  return (
    <ReactFlowProvider>
      <div className="space-y-4">
        <div className="flex flex-wrap items-center gap-3">
          <Input
            placeholder="Search schemas or handlers"
            value={search}
            onChange={(event) => setSearch(event.target.value)}
            className="w-64"
          />
          <div className="flex gap-1 rounded-full border bg-muted p-1 text-xs">
            {VIEW_OPTIONS.map((option) => (
              <button
                key={option.value}
                type="button"
                onClick={() => setView(option.value)}
                className={cn(
                  "rounded-full px-3 py-1 font-medium transition",
                  view === option.value
                    ? "bg-background text-foreground shadow-sm"
                    : "text-muted-foreground hover:text-foreground"
                )}
              >
                {option.label}
              </button>
            ))}
          </div>
          {(search || view !== "all") && (
            <Button
              variant="ghost"
              size="sm"
              onClick={() => {
                setSearch("");
                setView("all");
              }}
            >
              Reset
            </Button>
          )}
        </div>

        <div className="h-[78vh] min-h-[600px] w-full overflow-hidden">
          <PanelGroup direction="horizontal" className="h-full w-full">
            <Panel defaultSize={65} minSize={30} className="overflow-hidden">
              <div className="h-full w-full overflow-hidden">
                <SchemaGraphInner
                  nodes={filteredNodes}
                  edges={filteredEdges}
                  onNodeSelect={(nodeId) => setSelectedNodeId(nodeId)}
                  onClearSelection={() => setSelectedNodeId(null)}
                  highlight={highlight}
                />
              </div>
            </Panel>
            <PanelResizeHandle className="group relative w-2 shrink-0">
              <div className="absolute inset-y-0 left-1/2 w-1 -translate-x-1/2 rounded-full bg-border transition-colors group-hover:bg-primary/50 group-active:bg-primary" />
            </PanelResizeHandle>
            <Panel defaultSize={35} minSize={25} maxSize={50} className="overflow-hidden">
              <div className="h-full w-full overflow-hidden">
                <NodePanel
                  key={selectedNode?.id ?? "empty"}
                  node={selectedNode}
                  root={root}
                  edges={schemaEdges}
                  nodes={schemaNodes}
                />
              </div>
            </Panel>
          </PanelGroup>
        </div>
      </div>
    </ReactFlowProvider>
  );
}

function SchemaGraphInner({
  nodes: schemaNodes,
  edges: schemaEdges,
  onNodeSelect,
  onClearSelection,
  highlight,
}: {
  nodes: SchemaGraphNode[];
  edges: SchemaGraphData["edges"];
  onNodeSelect: (nodeId: string) => void;
  onClearSelection: () => void;
  highlight: { nodes: Set<string>; edges: Set<string> } | null;
}) {
  const { initialNodes, initialEdges } = useMemo(
    () => createLayout(schemaNodes, schemaEdges, highlight),
    [schemaNodes, schemaEdges, highlight]
  );
  const [nodes, setNodes, onNodesChange] = useNodesState(initialNodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState(initialEdges);

  useEffect(() => {
    const data = createLayout(schemaNodes, schemaEdges, highlight);
    setNodes(data.initialNodes);
    setEdges(data.initialEdges);
  }, [schemaNodes, schemaEdges, highlight, setNodes, setEdges]);

  return (
    <div className="h-full w-full rounded-xl border overflow-hidden">
      <ReactFlow
        nodes={nodes}
        edges={edges}
        nodeTypes={nodeTypes}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onNodeClick={(_, node) => onNodeSelect(node.id)}
        onPaneClick={onClearSelection}
        defaultEdgeOptions={{
          animated: true,
        }}
        fitView
        minZoom={1}
        maxZoom={1.5}
      >
        <Background gap={16} size={1} />
        {/* <MiniMap pannable zoomable /> */}
        <Controls />
      </ReactFlow>
    </div>
  );
}

function CustomNode({ data }: NodeProps) {
  const nodeData = data as {
    label: string;
    bucket: string;
    icon: React.ComponentType<{ size?: number; style?: React.CSSProperties }>;
    iconColor?: string;
    width?: number;
    height?: number;
    style?: React.CSSProperties;
  };
  
  const Icon = nodeData.icon;
  const iconColor = nodeData.iconColor || "hsl(var(--foreground))";
  
  return (
    <div
      className="relative"
      style={{
        width: nodeData.width ?? 220,
        height: nodeData.height ?? 80,
      }}
    >
      <Handle type="target" position={Position.Left} />
      <div
        style={{
          ...(nodeData.style || {}),
          display: "flex",
          alignItems: "center",
          gap: "10px",
          padding: "10px 14px",
        }}
      >
        {Icon && (
          <Icon
            size={20}
            style={{
              color: iconColor,
              flexShrink: 0,
            }}
          />
        )}
        <div style={{ flex: 1, minWidth: 0 }}>
          <div
            style={{
              fontSize: "13px",
              fontWeight: 600,
              color: "hsl(var(--foreground))",
              marginBottom: "4px",
              whiteSpace: "nowrap",
              overflow: "hidden",
              textOverflow: "ellipsis",
            }}
          >
            {nodeData.label}
          </div>
          <div
            style={{
              fontSize: "11px",
              color: "hsl(var(--muted-foreground))",
              whiteSpace: "nowrap",
              overflow: "hidden",
              textOverflow: "ellipsis",
            }}
          >
            {nodeData.bucket}
          </div>
        </div>
      </div>
      <Handle type="source" position={Position.Right} />
    </div>
  );
}

const nodeTypes = {
  custom: CustomNode,
};

function createLayout(
  nodes: SchemaGraphNode[],
  edges: SchemaGraphData["edges"],
  highlight: { nodes: Set<string>; edges: Set<string> } | null
) {
  const NODE_WIDTH = 220;
  const NODE_HEIGHT = 80;

  const dagreGraph = new dagre.graphlib.Graph();
  dagreGraph.setDefaultEdgeLabel(() => ({}));
  dagreGraph.setGraph({
    rankdir: "LR",
    nodesep: 50,
    ranksep: 120,
    align: "UL",
    acyclicer: "greedy",
    ranker: "tight-tree",
  });

  nodes.forEach((node) => {
    dagreGraph.setNode(node.id, {
      width: NODE_WIDTH,
      height: NODE_HEIGHT,
    });
  });

  edges.forEach((edge) => {
    dagreGraph.setEdge(edge.source, edge.target);
  });

  dagre.layout(dagreGraph);

  const positionedNodes = nodes.map((node) => {
    const dagreNode = dagreGraph.node(node.id);
    const position = {
      x: dagreNode.x - NODE_WIDTH / 2,
      y: dagreNode.y - NODE_HEIGHT / 2,
    };
    const isDimmed = highlight && !highlight.nodes.has(node.id);
    const isSelected = highlight && highlight.nodes.has(node.id);
    
    let Icon: React.ComponentType<{ size?: number; style?: React.CSSProperties }> = FileCode;
    let iconColor = "hsl(var(--muted-foreground))";
    
    if (node.type === "handler") {
      Icon = Code;
      iconColor = "hsl(var(--primary))";
    } else if (node.type === "schema") {
      Icon = Database;
      iconColor = "hsl(var(--foreground))";
    }
    
    const baseStyle =
      node.type === "handler"
        ? {
            border: "2px solid hsl(var(--primary))",
            background: "hsl(var(--primary)/0.1)",
            borderRadius: "14px",
          }
        : {
            border: "2px solid hsl(var(--border))",
            background: "hsl(var(--card))",
            borderRadius: "14px",
          };
    
    return {
      id: node.id,
      type: "custom",
      position,
      data: {
        label: node.label,
        bucket: node.bucket,
        icon: Icon,
        iconColor: iconColor,
        width: NODE_WIDTH,
        height: NODE_HEIGHT,
        style: {
          ...baseStyle,
          boxShadow: isSelected
            ? "0 0 0 3px hsl(var(--primary)/0.3)"
            : undefined,
          opacity: isDimmed ? 0.3 : 1,
          transition: "opacity 150ms ease",
        },
      },
    };
  });

  const rfEdges = edges.map((edge) => {
    const isDimmed = highlight && !highlight.edges.has(edge.id);
    return {
      id: edge.id,
      source: edge.source,
      target: edge.target,
      animated: true,
      label: edge.relation === "references" ? undefined : edge.relation,
      style:
        edge.relation === "triggers"
          ? { stroke: "hsl(var(--primary))", strokeWidth: 3, opacity: isDimmed ? 0.2 : 1 }
          : edge.relation === "handler"
            ? { stroke: "hsl(var(--secondary))", strokeWidth: 3, opacity: isDimmed ? 0.2 : 1 }
            : { strokeWidth: 2, opacity: isDimmed ? 0.2 : 1 },
      labelStyle: {
        fontSize: 10,
        fill: "hsl(var(--foreground))",
      },
      labelBgStyle: {
        fill: "hsl(var(--card))",
        stroke: "hsl(var(--border))",
        strokeWidth: 1,
      },
      labelBgPadding: [4, 6] as [number, number],
    };
  });

  return { initialNodes: positionedNodes, initialEdges: rfEdges };
}

function NodePanel({
  node,
  root,
  edges,
  nodes,
}: {
  node: SchemaGraphNode | null;
  root: string;
  edges: SchemaGraphData["edges"];
  nodes: SchemaGraphNode[];
}) {
  const [tab, setTab] = useState<"source" | "meta">("source");
  const nodeLookup = useMemo(
    () => new Map(nodes.map((n) => [n.id, n])),
    [nodes]
  );

  if (!node) {
    return (
      <div className="flex h-full items-center justify-center rounded-xl border p-4">
        <p className="text-sm text-muted-foreground text-center">
          Select a node to inspect schema or handler code.
        </p>
      </div>
    );
  }

  const outgoing = edges.filter((edge) => edge.source === node.id);
  const incoming = edges.filter((edge) => edge.target === node.id);

  const openInEditorUrl = `vscode://file/${encodeURI(`${root}/${node.path}`)}`;

  return (
    <div className="flex h-full w-full flex-col space-y-4 rounded-xl border p-4 overflow-hidden">
      <div className="flex items-center justify-between gap-2 min-w-0">
        <div className="min-w-0 flex-1">
          <p className="text-sm font-medium text-foreground truncate">{node.label}</p>
          <p className="text-xs text-muted-foreground truncate">{node.path}</p>
        </div>
        <Badge
          variant={node.type === "handler" ? "secondary" : "outline"}
          className="capitalize"
        >
          {node.type}
        </Badge>
      </div>

      <div className="flex gap-2 text-xs">
        {(["source", "meta"] as const).map((key) => (
          <button
            key={key}
            type="button"
            onClick={() => setTab(key)}
            className={cn(
              "rounded-full px-3 py-1 font-medium transition",
              tab === key
                ? "bg-primary text-primary-foreground"
                : "bg-muted text-muted-foreground"
            )}
          >
            {key === "source" ? "Source" : "Metadata"}
          </button>
        ))}
      </div>

      <div className="flex-1 overflow-hidden min-w-0 w-full">
        {tab === "source" ? (
          node.source ? (
            <SourceCodeViewer
              code={node.source}
              language={detectLanguage(node.path)}
              filePath={node.path}
            />
          ) : (
            <p className="text-sm text-muted-foreground">
              No source captured for this node.
            </p>
          )
        ) : (
          <div className="space-y-3 text-sm">
          <div className="flex items-center justify-between">
            <span className="text-muted-foreground">Bucket</span>
            <span className="font-medium text-foreground">{node.bucket}</span>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-muted-foreground">Type</span>
            <span className="font-medium text-foreground capitalize">
              {node.type}
            </span>
          </div>
          <div className="space-y-1">
            <p className="text-xs font-medium text-muted-foreground">
              Outgoing
            </p>
            {outgoing.length === 0 ? (
              <p className="text-xs text-muted-foreground">
                No outgoing relationships.
              </p>
            ) : (
              outgoing.map((edge) => (
                <p key={edge.id} className="text-xs text-muted-foreground">
                  {edge.relation} →{" "}
                  {nodeLookup.get(edge.target)?.label ?? edge.target}
                </p>
              ))
            )}
          </div>
          <div className="space-y-1">
            <p className="text-xs font-medium text-muted-foreground">
              Incoming
            </p>
            {incoming.length === 0 ? (
              <p className="text-xs text-muted-foreground">
                No incoming relationships.
              </p>
            ) : (
              incoming.map((edge) => (
                <p key={edge.id} className="text-xs text-muted-foreground">
                  {nodeLookup.get(edge.source)?.label ?? edge.source} →{" "}
                  {edge.relation}
                </p>
              ))
            )}
          </div>
          <div className="flex gap-2">
            <Button asChild variant="outline" size="sm" className="gap-1">
              <a href={openInEditorUrl}>Open in editor</a>
            </Button>
            <Button
              variant="ghost"
              size="sm"
              onClick={() => {
                navigator.clipboard.writeText(node.path).catch(() => {});
              }}
            >
              Copy path
            </Button>
          </div>
        </div>
        )}
      </div>
    </div>
  );
}

function SourceCodeViewer({
  code,
  language,
  filePath,
}: {
  code: string;
  language: string;
  filePath: string;
}) {
  const [copied, setCopied] = useState(false);

  const handleCopy = async () => {
    await navigator.clipboard.writeText(code);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div className="relative flex h-full w-full flex-col rounded-lg border overflow-hidden bg-[#1e1e1e] dark:bg-[#1e1e1e]">
      <div className="flex shrink-0 items-center justify-between border-b border-border/50 bg-muted/30 px-4 py-2.5">
        <div className="flex items-center gap-2 min-w-0">
          <span className="text-xs font-mono text-muted-foreground truncate" title={filePath}>
            {filePath}
          </span>
          <Badge variant="outline" className="text-xs shrink-0">
            {language}
          </Badge>
        </div>
        <Button
          variant="ghost"
          size="sm"
          onClick={handleCopy}
          className="h-7 gap-1.5 text-xs shrink-0"
        >
          {copied ? (
            <>
              <Check className="h-3.5 w-3.5" />
              Copied
            </>
          ) : (
            <>
              <Copy className="h-3.5 w-3.5" />
              Copy
            </>
          )}
        </Button>
      </div>
      <div className="flex-1 overflow-auto w-full [&::-webkit-scrollbar]:w-2 [&::-webkit-scrollbar]:h-2 [&::-webkit-scrollbar-thumb]:rounded-full [&::-webkit-scrollbar-thumb]:bg-muted-foreground/20 hover:[&::-webkit-scrollbar-thumb]:bg-muted-foreground/30">
        <SyntaxHighlighter
          language={language}
          style={vscDarkPlus}
          customStyle={{
            margin: 0,
            padding: "1rem",
            fontSize: "12px",
            lineHeight: "1.6",
            background: "transparent",
            fontFamily: "ui-monospace, SFMono-Regular, 'SF Mono', Menlo, Consolas, 'Liberation Mono', monospace",
            width: "100%",
            minWidth: "100%",
          }}
          showLineNumbers
          lineNumberStyle={{
            minWidth: "3.5em",
            paddingRight: "1.5em",
            paddingLeft: "0.5em",
            color: "#6e768166",
            userSelect: "none",
            textAlign: "right",
          }}
          wrapLines={false}
          wrapLongLines={false}
          PreTag={({ children, ...props }) => (
            <div {...props} className="w-full" style={{ width: "100%" }}>
              {children}
            </div>
          )}
        >
          {code}
        </SyntaxHighlighter>
      </div>
    </div>
  );
}

function detectLanguage(filePath: string): string {
  const ext = filePath.split(".").pop()?.toLowerCase() ?? "";
  const languageMap: Record<string, string> = {
    ts: "typescript",
    tsx: "tsx",
    js: "javascript",
    jsx: "jsx",
    py: "python",
    rs: "rust",
    ro: "rust",
    toml: "toml",
    json: "json",
    yaml: "yaml",
    yml: "yaml",
    md: "markdown",
    html: "html",
    css: "css",
    scss: "scss",
    sql: "sql",
    sh: "bash",
    bash: "bash",
    zsh: "bash",
  };
  return languageMap[ext] ?? "text";
}
