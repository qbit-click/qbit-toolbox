import { createMD3Theme } from "md3-next";

export const qbitSeed = "#6750A4";
export type ThemePreference = "system" | "light" | "dark";

export const qbitTheme = createMD3Theme({ seed: qbitSeed, mode: "system" });
