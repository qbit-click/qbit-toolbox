"use client";

import { QbitThemeProvider, type QbitThemeProviderProps } from "@qbit/ui";
import { I18nProvider, type CatalogExtensions } from "@qbit/i18n";

export interface ProvidersProps extends Pick<
  QbitThemeProviderProps,
  "children"
> {
  catalogExtensions?: CatalogExtensions;
}

export function Providers({ children, catalogExtensions }: ProvidersProps) {
  return (
    <I18nProvider catalogExtensions={catalogExtensions}>
      <QbitThemeProvider>{children}</QbitThemeProvider>
    </I18nProvider>
  );
}
