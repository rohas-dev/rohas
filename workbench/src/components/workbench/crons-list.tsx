"use client";

import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Clock, Calendar } from "lucide-react";
import type { CronJob } from "@/lib/workbench-data";

export function CronsList({ crons }: { crons: CronJob[] }) {
  if (crons.length === 0) {
    return (
      <Card>
        <CardHeader>
          <CardTitle>Cron Jobs</CardTitle>
          <CardDescription>No cron jobs found in your schema</CardDescription>
        </CardHeader>
        <CardContent>
          <p className="text-sm text-muted-foreground">
            Add cron jobs to your schema files to see them listed here.
          </p>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>Cron Jobs</CardTitle>
        <CardDescription>Scheduled tasks defined in your schema</CardDescription>
      </CardHeader>
      <CardContent>
        <div className="space-y-4">
          {crons.map((cron) => (
            <div
              key={cron.name}
              className="flex items-start justify-between p-4 rounded-lg border bg-card"
            >
              <div className="flex-1 space-y-2">
                <div className="flex items-center gap-2">
                  <h3 className="font-semibold">{cron.name}</h3>
                  <Badge variant="outline" className="text-xs">
                    <Clock className="h-3 w-3 mr-1" />
                    {cron.schedule}
                  </Badge>
                </div>
                {cron.triggers.length > 0 && (
                  <div className="flex flex-wrap gap-2">
                    <span className="text-xs text-muted-foreground">Triggers:</span>
                    {cron.triggers.map((trigger, idx) => (
                      <Badge key={idx} variant="secondary" className="text-xs">
                        {trigger}
                      </Badge>
                    ))}
                  </div>
                )}
              </div>
            </div>
          ))}
        </div>
      </CardContent>
    </Card>
  );
}

