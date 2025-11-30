import { Terminal } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from "@/components/ui/card";

const tasks = [
  {
    label: "Generate SDK",
    description: "Compile TypeScript clients based on the latest schema bundle.",
    command: "rohas codegen",
  },
  {
    label: "Validate schema bundle",
    description: "Ensure every .ro file is syntactically and semantically valid.",
    command: "rohas validate ./schema",
  },
  {
    label: "Start development server",
    description: "Launch adapters, workflows, and WebSocket entry points locally.",
    command: "rohas dev --port 4000",
  },
];

export default function CliPage() {
  return (
    <div className="space-y-8">
      <Card>
        <CardHeader>
          <CardTitle>CLI playbook</CardTitle>
          <CardDescription>Keep frequently used scripts one click away.</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {tasks.map((task) => (
            <div key={task.command} className="rounded-lg border bg-muted/30 p-4 text-sm text-muted-foreground">
              <p className="flex items-center gap-2 text-base font-medium text-foreground">
                <Terminal className="h-4 w-4" />
                {task.label}
              </p>
              <p className="mt-1">{task.description}</p>
              <code className="mt-2 block text-xs">{task.command}</code>
            </div>
          ))}
        </CardContent>
        <CardFooter>
          <Button className="gap-2" variant="outline">
            <Terminal className="h-4 w-4" />
            Open interactive shell
          </Button>
        </CardFooter>
      </Card>
    </div>
  );
}

