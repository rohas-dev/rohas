"use client";

import { useMemo } from "react";

import { Badge } from "@/components/ui/badge";
import {
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from "@/components/ui/tabs";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { useWorkbenchStore } from "@/stores/workbench-store";
import type { EntityRow } from "@/types/workbench";

type Props = {
  schemaRows: EntityRow[];
  handlerRows: EntityRow[];
};

export function SchemaBrowser({ schemaRows, handlerRows }: Props) {
  const search = useWorkbenchStore((state) => state.search);

  const filteredSchemas = useFilteredRows(schemaRows, search);
  const filteredHandlers = useFilteredRows(handlerRows, search);

  return (
    <Tabs defaultValue="schemas">
      <TabsList>
        <TabsTrigger value="schemas">Schemas</TabsTrigger>
        <TabsTrigger value="handlers">Handlers</TabsTrigger>
      </TabsList>
      <TabsContent value="schemas">
        <EntityTable rows={filteredSchemas} emptyMessage="Add .ro files under /schema to see them here." />
      </TabsContent>
      <TabsContent value="handlers">
        <EntityTable
          rows={filteredHandlers}
          emptyMessage="Add handler implementations under /src/handlers."
        />
      </TabsContent>
    </Tabs>
  );
}

function EntityTable({ rows, emptyMessage }: { rows: EntityRow[]; emptyMessage: string }) {
  if (rows.length === 0) {
    return <p className="text-sm text-muted-foreground">{emptyMessage}</p>;
  }

  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>Name</TableHead>
          <TableHead>Group</TableHead>
          <TableHead>Path</TableHead>
          <TableHead className="text-right">Size</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {rows.map((row) => (
          <TableRow key={row.path}>
            <TableCell className="font-medium text-foreground">{row.name}</TableCell>
            <TableCell>
              <Badge variant="outline" className="capitalize">
                {row.bucket}
              </Badge>
            </TableCell>
            <TableCell className="text-xs text-muted-foreground">{row.path}</TableCell>
            <TableCell className="text-right text-xs text-muted-foreground">{row.size}</TableCell>
          </TableRow>
        ))}
      </TableBody>
    </Table>
  );
}

function useFilteredRows(rows: EntityRow[], search: string) {
  return useMemo(() => {
    if (!search) return rows;
    const query = search.toLowerCase();
    return rows.filter(
      (row) =>
        row.name.toLowerCase().includes(query) ||
        row.bucket.toLowerCase().includes(query) ||
        row.path.toLowerCase().includes(query),
    );
  }, [rows, search]);
}

