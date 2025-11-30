"use client";

import { create } from "zustand";
import { persist } from "zustand/middleware";

export type TraceType = "api" | "event" | "cron" | "websocket";

type TimelineFiltersState = {
  showFilters: boolean;
  selectedTypes: TraceType[];
  startTime: string;
  endTime: string;
  zoom: number;
  
  // Actions
  setShowFilters: (show: boolean) => void;
  toggleType: (type: TraceType) => void;
  setSelectedTypes: (types: TraceType[]) => void;
  setStartTime: (time: string) => void;
  setEndTime: (time: string) => void;
  setZoom: (zoom: number) => void;
  clearFilters: () => void;
};

const defaultTypes: TraceType[] = ["api", "event", "cron", "websocket"];

export const useTimelineFiltersStore = create<TimelineFiltersState>()(
  persist(
    (set) => ({
      showFilters: false,
      selectedTypes: defaultTypes,
      startTime: "",
      endTime: "",
      zoom: 0.5, // DEFAULT_PIXELS_PER_MS

      setShowFilters: (show) => set({ showFilters: show }),
      
      toggleType: (type) =>
        set((state) => {
          const newTypes = state.selectedTypes.includes(type)
            ? state.selectedTypes.filter((t) => t !== type)
            : [...state.selectedTypes, type];
          return { selectedTypes: newTypes };
        }),

      setSelectedTypes: (types) => set({ selectedTypes: types }),

      setStartTime: (time) => set({ startTime: time }),

      setEndTime: (time) => set({ endTime: time }),

      setZoom: (zoom) => set({ zoom }),

      clearFilters: () =>
        set({
          selectedTypes: defaultTypes,
          startTime: "",
          endTime: "",
        }),
    }),
    {
      name: "timeline-filters-storage",
    }
  )
);

