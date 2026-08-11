"use client";

import { AppRouterCacheProvider } from "@mui/material-nextjs/v16-appRouter";
import type { ReactNode } from "react";

export function resolveRuntimeCspNonce(): string | undefined {
  if (typeof document === "undefined") {
    return undefined;
  }

  const styleNonce = document
    .querySelector<HTMLStyleElement>("style[nonce]")
    ?.nonce.trim();

  if (styleNonce) {
    return styleNonce;
  }

  const scriptNonce = document
    .querySelector<HTMLScriptElement>("script[nonce]")
    ?.nonce.trim();

  if (scriptNonce) {
    return scriptNonce;
  }

  return undefined;
}

export function StyleCacheProvider({ children }: { children: ReactNode }) {
  const nonce = resolveRuntimeCspNonce();

  return (
    <AppRouterCacheProvider options={nonce ? { nonce } : undefined}>
      {children}
    </AppRouterCacheProvider>
  );
}
