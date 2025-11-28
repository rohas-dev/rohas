import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { SchemaBrowser } from "@/components/workbench/schema-browser";
import { fetchWorkbenchData } from "@/lib/workbench-data";

export const revalidate = 0;

export default async function SchemasPage() {
  const data = await fetchWorkbenchData();

  return (
    <div className="space-y-8">
      <section>
        <Card>
          <CardHeader>
            <CardTitle>Schemas & handlers</CardTitle>
            <CardDescription>
              Use the global search bar to filter by file name, bucket, or relative path.
            </CardDescription>
          </CardHeader>
          <CardContent>
            <SchemaBrowser schemaRows={data.schemaRows} handlerRows={data.handlerRows} />
          </CardContent>
        </Card>
      </section>

      <section className="grid gap-6 lg:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle>Tips</CardTitle>
            <CardDescription>Keep your schema inventory healthy.</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4 text-sm text-muted-foreground">
            <p>
              Run <code>rohas validate</code> before committing to ensure schemas, adapters, and
              workflows remain consistent.
            </p>
            <p>
              Store shared DTOs inside <code>schema/models</code> so Workbench can surface them next
              to handlers.
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Watch mode</CardTitle>
            <CardDescription>Automate feedback as you edit .ro files.</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4 text-sm text-muted-foreground">
            <p>Use `rohas dev --watch` to revalidate and regenerate SDKs automatically.</p>
            <p>
              The Workbench UI polls the filesystem every time you refresh, so you’ll see updates as
              soon as the CLI finishes.
            </p>
          </CardContent>
        </Card>
      </section>
    </div>
  );
}

