import { SchemaGraph } from "@/components/workbench/schema-graph";
import { fetchSchemaGraph } from "@/lib/workbench-data";

export const revalidate = 0;

export default async function SchemaGraphPage() {
  const graph = await fetchSchemaGraph();

  return (
    <div className="space-y-8">
      <section>
        <SchemaGraph {...graph} />
      </section>
    </div>
  );
}
