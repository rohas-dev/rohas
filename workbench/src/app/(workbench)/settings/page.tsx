import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { fetchWorkbenchData } from "@/lib/workbench-data";

export const revalidate = 0;

export default async function SettingsPage() {
  const data = await fetchWorkbenchData();
  const adapterEntries = Object.entries(data.config?.adapter ?? {}).filter(
    ([key]) => key !== "type",
  );

  return (
    <div className="space-y-8">
      <section className="grid gap-6 lg:grid-cols-3">
        <SettingsCard
          title="Project"
          description="Metadata sourced from config/rohas.toml."
          rows={[
            { label: "Name", value: data.config?.project?.name ?? "—" },
            { label: "Version", value: data.config?.project?.version ?? "—" },
            { label: "Language", value: data.config?.project?.language ?? "—" },
          ]}
        />
        <SettingsCard
          title="Server"
          description="Dev server defaults."
          rows={[
            { label: "Host", value: data.config?.server?.host ?? "—" },
            { label: "Port", value: data.config?.server?.port?.toString() ?? "—" },
            {
              label: "Endpoint",
              value:
                data.config?.server?.host && data.config?.server?.port
                  ? `http://${data.config.server.host}:${data.config.server.port}`
                  : "—",
            },
          ]}
        />
        <SettingsCard
          title="Adapter"
          description="Active transport configuration."
          rows={[
            { label: "Type", value: String(data.config?.adapter?.type ?? "—") },
            ...adapterEntries.map(([key, value]) => ({
              label: key,
              value: String(value ?? "—"),
            })),
          ]}
        />
      </section>

      <section>
        <Card>
          <CardHeader>
            <CardTitle>Environment overrides</CardTitle>
            <CardDescription>
              Workbench walks up the filesystem until it finds <code>config/rohas.toml</code>. Set a
              custom root to inspect another project.
            </CardDescription>
          </CardHeader>
          <CardContent className="text-sm text-muted-foreground">
            <code>ROHAS_PROJECT_ROOT=/absolute/path/to/project pnpm dev</code>
            <p className="mt-3">
              This is especially helpful when you install the Workbench bundle globally or keep
              multiple Rohas projects checked out at once.
            </p>
          </CardContent>
        </Card>
      </section>
    </div>
  );
}

function SettingsCard({
  title,
  description,
  rows,
}: {
  title: string;
  description: string;
  rows: Array<{ label: string; value: string }>;
}) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>{title}</CardTitle>
        <CardDescription>{description}</CardDescription>
      </CardHeader>
      <CardContent className="space-y-2 text-sm">
        {rows.map((row) => (
          <div key={row.label} className="flex items-center justify-between">
            <span className="text-muted-foreground">{row.label}</span>
            <span className="font-medium text-foreground">{row.value}</span>
          </div>
        ))}
      </CardContent>
    </Card>
  );
}

