"use client";

import {
  createContext,
  createElement,
  type ReactNode,
  useContext,
  useEffect,
  useMemo,
} from "react";

export type LocaleDirection = "ltr" | "rtl";
export type TranslationParameter = string | number;
export type TranslationCatalog = Readonly<Record<string, string>>;

export const englishLocale = { locale: "en", direction: "ltr" } as const;

export const englishCoreCatalog: TranslationCatalog = {
  "app.title": "Qbit Toolbox",
  "theme.preference": "Theme preference",
  "theme.system": "System",
  "theme.light": "Light",
  "theme.dark": "Dark",
  "core.status.heading": "Core status",
  "core.status.loading": "Loading core status…",
  "core.status.load_error": "Unable to load core status. Please try again.",
  "core.status.retry": "Retry",
  "core.recovery.title": "Recovery required",
  "core.recovery.message":
    "The application requires recovery before it can continue safely.",
  "core.app_version": "App version: {version}",
  "core.platform": "Platform: {platform}",
  "core.schema_version": "Schema version: {version}",
  "core.schema_unavailable": "Unavailable",
  "core.features.empty": "No features are registered yet.",
  "core.features.list_label": "Registered features",
  "core.feature.format": "{name} ({id}) — {lifecycle}",
  "lifecycle.unavailable": "Unavailable",
  "lifecycle.disabled": "Disabled",
  "lifecycle.starting": "Starting",
  "lifecycle.running": "Running",
  "lifecycle.degraded": "Degraded",
  "lifecycle.stopping": "Stopping",
  "lifecycle.failed": "Failed",
  "error.internal": "An internal error occurred.",
  "errors.core.migration_failed": "A data migration failed.",
  "errors.core.schema_unsupported": "The data schema is unsupported.",
  "errors.core.schema_inconsistent": "The data schema is inconsistent.",
  "errors.core.persistence_unavailable": "Persistent storage is unavailable.",
  "errors.core.schema_invalid": "The data schema is invalid.",
};

type TranslationParameters = Readonly<Record<string, TranslationParameter>>;
export type CatalogExtensions =
  TranslationCatalog | readonly TranslationCatalog[];

interface I18nValue {
  locale: string;
  direction: LocaleDirection;
  t: (key: string, params?: TranslationParameters) => string;
}

const I18nContext = createContext<I18nValue | null>(null);

export interface I18nProviderProps {
  children: ReactNode;
  locale?: "en";
  catalogExtensions?: CatalogExtensions;
}

function interpolate(template: string, params?: TranslationParameters): string {
  return template.replace(/\{([A-Za-z0-9_]+)\}/g, (token, name: string) => {
    const value = params?.[name];
    return value === undefined ? token : String(value);
  });
}

export function I18nProvider({
  children,
  locale = "en",
  catalogExtensions,
}: I18nProviderProps) {
  const catalog = useMemo(() => {
    const extensions =
      catalogExtensions === undefined
        ? []
        : Array.isArray(catalogExtensions)
          ? catalogExtensions
          : [catalogExtensions];
    return Object.assign(
      {},
      englishCoreCatalog,
      ...extensions,
    ) as TranslationCatalog;
  }, [catalogExtensions]);

  const direction = englishLocale.direction;
  useEffect(() => {
    if (typeof document !== "undefined") {
      document.documentElement.lang = locale;
      document.documentElement.dir = direction;
    }
  }, [direction, locale]);

  const value = useMemo<I18nValue>(
    () => ({
      locale,
      direction,
      t: (key, params) => interpolate(catalog[key] ?? key, params),
    }),
    [catalog, direction, locale],
  );

  return createElement(I18nContext.Provider, { value }, children);
}

export function useI18n(): I18nValue {
  const value = useContext(I18nContext);
  if (value === null)
    throw new Error("useI18n must be used within an I18nProvider");
  return value;
}
