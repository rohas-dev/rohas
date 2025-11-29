"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { Boxes, Code2, LayoutDashboard, Settings, Workflow, ChevronLeft, ChevronRight, Network } from "lucide-react";
import { ActivitySquare } from "lucide-react";
import { useEffect } from "react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Separator } from "@/components/ui/separator";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";
import { useWorkbenchStore, type View } from "@/stores/workbench-store";

const navItems: Array<{
  label: string;
  href: string;
  icon: typeof LayoutDashboard;
  view: View;
}> = [
  { label: "Overview", href: "/overview", icon: LayoutDashboard, view: "overview" },
  { label: "Schemas", href: "/schemas", icon: Boxes, view: "schemas" },
  { label: "Schema Graph", href: "/schema-graph", icon: Workflow, view: "schema-graph" },
  { label: "Tracing", href: "/tracing", icon: ActivitySquare, view: "tracing" },
  { label: "API & WebSocket", href: "/endpoints", icon: Network, view: "endpoints" },
  { label: "CLI Tasks", href: "/cli", icon: Code2, view: "cli" },
  { label: "Settings", href: "/settings", icon: Settings, view: "settings" },
];

export function Sidebar() {
  const pathname = usePathname();
  const setView = useWorkbenchStore((state) => state.setView);
  const sidebarCollapsed = useWorkbenchStore((state) => state.sidebarCollapsed);
  const toggleSidebar = useWorkbenchStore((state) => state.toggleSidebar);

  useEffect(() => {
    const current = navItems.find((item) => item.href === pathname);
    if (current) {
      setView(current.view);
    }
  }, [pathname, setView]);

  return (
    <aside
      className={cn(
        "sticky top-0 hidden h-screen flex-col border-r bg-card/40 overflow-y-auto transition-all duration-300 lg:flex",
        sidebarCollapsed ? "w-16 p-3" : "w-64 p-6"
      )}
    >
      <div className="flex items-center justify-between mb-6">
        {!sidebarCollapsed && (
          <div className="space-y-1">
            <p className="text-xs uppercase tracking-widest text-muted-foreground">Workspace</p>
            <p className="text-lg font-semibold">Rohas / Core</p>
            <Badge variant="outline" className="mt-2 capitalize">
              Synced · local
            </Badge>
          </div>
        )}
        <Button
          variant="ghost"
          size="icon"
          onClick={toggleSidebar}
          className="ml-auto h-8 w-8 shrink-0"
        >
          {sidebarCollapsed ? (
            <ChevronRight className="h-4 w-4" />
          ) : (
            <ChevronLeft className="h-4 w-4" />
          )}
        </Button>
      </div>

      {!sidebarCollapsed && <Separator className="my-6" />}

      <nav className="flex flex-1 flex-col gap-1">
        <TooltipProvider delayDuration={0}>
          {navItems.map((item) => {
            const isActive = pathname === item.href;
            const linkContent = (
              <Link
                href={item.href}
                className={cn(
                  "flex items-center gap-2 rounded-md px-3 py-2 text-sm transition",
                  sidebarCollapsed ? "justify-center px-2" : "",
                  isActive
                    ? "bg-accent text-accent-foreground"
                    : "text-muted-foreground hover:bg-accent hover:text-accent-foreground"
                )}
              >
                <item.icon className="h-4 w-4 shrink-0" />
                {!sidebarCollapsed && <span>{item.label}</span>}
              </Link>
            );

            if (sidebarCollapsed) {
              return (
                <Tooltip key={item.href}>
                  <TooltipTrigger asChild>{linkContent}</TooltipTrigger>
                  <TooltipContent side="right">
                    <p>{item.label}</p>
                  </TooltipContent>
                </Tooltip>
              );
            }

            return <div key={item.href}>{linkContent}</div>;
          })}
        </TooltipProvider>
      </nav>

      {!sidebarCollapsed && (
        <div className="rounded-lg border border-dashed bg-muted/30 p-4 text-sm text-muted-foreground">
          <p className="font-medium text-foreground">Need more adapters?</p>
          <p className="mb-3 text-xs">
            Scaffold new transport layers or templates straight from Workbench.
          </p>
          <a
            href="https://github.com/rohas-dev/rohas"
            className="text-xs font-medium text-primary hover:underline"
          >
            View contributor guide →
          </a>
        </div>
      )}
    </aside>
  );
}

