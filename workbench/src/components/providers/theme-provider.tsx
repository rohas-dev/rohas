"use client";

import { ThemeProvider as NextThemeProvider } from "next-themes";
import * as React from "react";

export function ThemeProvider({ children }: { children: React.ReactNode }) {
  return (
    <NextThemeProvider attribute="class" defaultTheme="dark" enableSystem disableTransitionOnChange>
      {children}
    </NextThemeProvider>
  );
}

