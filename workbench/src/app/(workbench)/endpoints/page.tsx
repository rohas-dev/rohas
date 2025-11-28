"use client";

import { useEffect, useState } from "react";
import { ApiTester } from "@/components/workbench/api-tester";
import { CronsList } from "@/components/workbench/crons-list";
import { EventsTester } from "@/components/workbench/events-tester";
import { fetchEndpoints, type EndpointsData } from "@/lib/workbench-data";
import { Loader2 } from "lucide-react";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";

export default function EndpointsPage() {
  const [data, setData] = useState<EndpointsData | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    fetchEndpoints().then((endpoints) => {
      setData(endpoints);
      setLoading(false);
    });
  }, []);

  if (loading) {
    return (
      <div className="flex items-center justify-center min-h-[400px]">
        <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
      </div>
    );
  }

  if (!data) {
    return (
      <div className="space-y-8">
        <div>
          <h1 className="text-3xl font-bold">API & WebSocket Testing</h1>
          <p className="text-muted-foreground mt-2">
            Test your API endpoints and WebSocket connections
          </p>
        </div>
        <p className="text-sm text-muted-foreground">Failed to load endpoints data.</p>
      </div>
    );
  }

  return (
    <div className="space-y-8">
      <div>
        <h1 className="text-3xl font-bold">API & WebSocket Testing</h1>
        <p className="text-muted-foreground mt-2">
          Test your API endpoints, WebSocket connections, trigger events, and view scheduled cron jobs
        </p>
      </div>

      <Tabs defaultValue="api" className="w-full">
        <TabsList className="grid w-full grid-cols-3">
          <TabsTrigger value="api">API & WebSocket</TabsTrigger>
          <TabsTrigger value="events">Events</TabsTrigger>
          <TabsTrigger value="crons">Cron Jobs</TabsTrigger>
        </TabsList>

        <TabsContent value="api" className="space-y-4">
          <ApiTester apis={data.apis} websockets={data.websockets} />
        </TabsContent>

        <TabsContent value="events" className="space-y-4">
          <EventsTester events={data.events} />
        </TabsContent>

        <TabsContent value="crons" className="space-y-4">
          <CronsList crons={data.crons} />
        </TabsContent>
      </Tabs>
    </div>
  );
}

