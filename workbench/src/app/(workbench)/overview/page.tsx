import {
  GitBranch,
  Layers,
  PlugZap,
  Server,
  Terminal,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Progress } from "@/components/ui/progress";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Separator } from "@/components/ui/separator";
import { fetchWorkbenchData } from "@/lib/workbench-data";
import type { ProjectConfig } from "@/lib/project";
import type { ActivityItem } from "@/types/workbench";
import { SystemMetrics } from "@/components/workbench/system-metrics";

export const revalidate = 0;

export default async function OverviewPage() {
  const data = await fetchWorkbenchData();

  const insightCards = [
    {
      label: "Project",
      value: data.config?.project?.name ?? "Unnamed",
      description: `v${data.config?.project?.version ?? "0.0.0"}`,
      icon: Layers,
    },
    {
      label: "Language",
      value: data.config?.project?.language ?? "n/a",
      description: "Generator target",
      icon: Terminal,
    },
    {
      label: "Schema files",
      value: data.schemaTotal,
      description: `${data.schemaBucketCount} categories`,
      icon: GitBranch,
    },
    {
      label: "Handlers",
      value: data.handlerTotal,
      description: formatAdapterDescription(data.config),
      icon: PlugZap,
    },
  ];

  return (
    <div className="space-y-8">
      <section className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
        {insightCards.map((card) => (
          <Card key={card.label}>
            <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
              <CardDescription>{card.label}</CardDescription>
              <card.icon className="h-4 w-4 text-muted-foreground" />
            </CardHeader>
            <CardContent>
              <div className="text-2xl font-semibold">{card.value}</div>
              <p className="text-sm text-muted-foreground">{card.description}</p>
            </CardContent>
          </Card>
        ))}
      </section>

      <section className="grid gap-6 lg:grid-cols-3">
        <SystemMetrics />

        <Card>
          <CardHeader>
            <CardTitle>Runtime posture</CardTitle>
            <CardDescription>Signals pulled from config/rohas.toml.</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <RuntimeBar label="Schema coverage" value={Math.min(100, data.schemaTotal * 5)} valueLabel={String(data.schemaTotal)} />
            <RuntimeBar label="Handlers available" value={Math.min(100, data.handlerTotal * 5)} valueLabel={String(data.handlerTotal)} />
            <RuntimeBar
              label="Adapter"
              value={66}
              valueLabel={formatAdapterLabel(data.config) ?? "—"}
            />
          </CardContent>
          <CardFooter className="flex flex-col items-start gap-2 text-sm text-muted-foreground">
            <span>Use `rohas dev` to spin up the local engine with hot reload.</span>
            <Button size="sm" className="gap-2" variant="secondary">
              <Server className="h-4 w-4" />
              {data.config?.server
                ? `Server ${data.config.server.host}:${data.config.server.port}`
                : "No server configured"}
            </Button>
          </CardFooter>
        </Card>
      </section>

      <section className="grid gap-6 lg:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle>Recent activity</CardTitle>
            <CardDescription>Deterministic log derived from project files.</CardDescription>
          </CardHeader>
          <CardContent>
            <ScrollArea className="h-72">
              <div className="space-y-4 pr-4">
                {data.activity.map((item) => (
                  <ActivityRow key={item.id} item={item} />
                ))}
              </div>
            </ScrollArea>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Quick actions</CardTitle>
            <CardDescription>Common workflows to keep your project in sync.</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            {quickActions.map((action) => (
              <div
                key={action.title}
                className="rounded-lg border bg-muted/30 p-4 text-sm text-muted-foreground"
              >
                <p className="mb-1 font-medium text-foreground">{action.title}</p>
                <p>{action.description}</p>
                <code className="mt-2 block text-xs">{action.command}</code>
              </div>
            ))}
          </CardContent>
        </Card>
      </section>
    </div>
  );
}

function RuntimeBar({
  label,
  value,
  valueLabel,
}: {
  label: string;
  value: number;
  valueLabel?: string;
}) {
  return (
    <div>
      <div className="mb-2 flex items-center justify-between text-sm font-medium">
        <span>{label}</span>
        <span>{valueLabel ?? value}</span>
      </div>
      <Progress value={value} />
    </div>
  );
}

function ActivityRow({ item }: { item: ActivityItem }) {
  return (
    <div>
      <div className="flex items-center justify-between text-sm font-medium">
        <span>{item.title}</span>
        <span className="text-xs text-muted-foreground">{item.timestamp}</span>
      </div>
      <p className="text-sm text-muted-foreground">{item.description}</p>
      <Separator className="mt-3" />
    </div>
  );
}

const quickActions = [
  {
    title: "Refresh generated SDKs",
    description: "Rebuild TypeScript clients after editing schema files.",
    command: "rohas codegen --lang ts",
  },
  {
    title: "Verify schema bundle",
    description: "Validate your project state before pushing to CI.",
    command: "rohas validate ./schema",
  },
  {
    title: "Start dev server",
    description: "Run adapters, workflows, and WS endpoints with live reload.",
    command: "rohas dev --port 4000",
  },
];

function formatAdapterDescription(config?: ProjectConfig) {
  const adapterType = formatAdapterLabel(config);
  return adapterType ? `${adapterType} adapter` : "No adapter configured";
}

function formatAdapterLabel(config?: ProjectConfig) {
  const adapter = config?.adapter;
  if (adapter && typeof adapter.type === "string") {
    return adapter.type as string;
  }
  return undefined;
}

