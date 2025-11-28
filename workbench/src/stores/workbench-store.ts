"use client";

import { create } from "zustand";

export type View =
  | "overview"
  | "schemas"
  | "schema-graph"
  | "tracing"
  | "cli"
  | "settings"
  | "endpoints";

type WorkbenchState = {
  view: View;
  search: string;
  sidebarCollapsed: boolean;
  setView: (view: View) => void;
  setSearch: (value: string) => void;
  toggleSidebar: () => void;
};

export const useWorkbenchStore = create<WorkbenchState>((set) => ({
  view: "overview",
  search: "",
  sidebarCollapsed: false,
  setView: (view) => set({ view }),
  setSearch: (search) => set({ search }),
  toggleSidebar: () => set((state) => ({ sidebarCollapsed: !state.sidebarCollapsed })),
}));

