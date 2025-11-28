import type { ReactNode } from "react";

import { Header } from "@/components/workbench/header";
import { Sidebar } from "@/components/workbench/sidebar";

export default function WorkbenchLayout({ children }: { children: ReactNode }) {
  return (
    <div className="min-h-screen bg-background text-foreground">
      <div className="flex">
        <Sidebar />
        <div className="flex flex-1 flex-col min-w-0">
          <Header />
          <main className="flex-1 space-y-8 p-6">{children}</main>
        </div>
      </div>
    </div>
  );
}

