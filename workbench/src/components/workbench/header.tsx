"use client";

import { Bell, Flame, Search, Terminal } from "lucide-react";
import Link from "next/link";

import { ThemeToggle } from "@/components/theme-toggle";
import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { useWorkbenchStore } from "@/stores/workbench-store";

export function Header() {
  const search = useWorkbenchStore((state) => state.search);
  const setSearch = useWorkbenchStore((state) => state.setSearch);

  return (
    <header className="flex flex-col gap-4 border-b bg-background/80 p-4 backdrop-blur supports-[backdrop-filter]:bg-background/60 md:flex-row md:items-center md:justify-between">
      <div className="space-y-2">
        <div className="flex items-center gap-2 text-xs text-muted-foreground">
          <Badge variant="success" className="flex items-center gap-1">
            <Flame className="h-3 w-3" />
            Live preview
          </Badge>
          <span>commit fb23c1e · synced 2m ago</span>
        </div>
        <div className="flex flex-wrap items-center gap-3">
          <h1 className="text-2xl font-semibold tracking-tight">Workbench</h1>
          <span className="rounded-full border px-3 py-1 text-xs text-muted-foreground">
            app/workbench@v0.1.0
          </span>
        </div>
        <p className="text-sm text-muted-foreground">
          Inspect crates, orchestrate schema changes, and run CLI workflows without leaving the
          browser.
        </p>
      </div>

      <div className="flex w-full flex-col gap-3 md:w-auto md:flex-row md:items-center">
        <div className="flex items-center justify-end gap-2">
          <ThemeToggle />
          <Button variant="outline" className="gap-2">
            <Terminal className="h-4 w-4" />
            Open CLI
          </Button>
          <Button className="gap-2" asChild>
            <Link href="https://github.com/rohas-dev/rohas">
              <Bell className="h-4 w-4" />
              Notify me
            </Link>
          </Button>
        </div>
      </div>
    </header>
  );
}

