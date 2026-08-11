import { StyleCacheProvider } from "./style-cache-provider";
import type { Metadata } from "next";
import type { ReactNode } from "react";

import { Providers } from "./providers";

export const metadata: Metadata = {
  title: "Qbit Toolbox",
  description: "A foundation for Qbit desktop tools.",
};

export default function RootLayout({
  children,
}: Readonly<{ children: ReactNode }>) {
  return (
    <html lang="en" dir="ltr" suppressHydrationWarning>
      <body>
        <StyleCacheProvider>
          <Providers>{children}</Providers>
        </StyleCacheProvider>
      </body>
    </html>
  );
}
