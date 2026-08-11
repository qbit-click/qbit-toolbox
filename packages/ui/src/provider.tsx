"use client";

import { MD3Provider } from "md3-next";
import { type ReactNode } from "react";

import { qbitTheme } from "./theme";

export interface QbitThemeProviderProps {
  children: ReactNode;
}

export function QbitThemeProvider({ children }: QbitThemeProviderProps) {
  return <MD3Provider theme={qbitTheme}>{children}</MD3Provider>;
}
