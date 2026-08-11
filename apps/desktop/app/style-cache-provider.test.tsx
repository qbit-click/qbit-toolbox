import type { ReactNode } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";

interface AppRouterCacheProviderProps {
  children: ReactNode;
  options?: {
    nonce?: string;
  };
}

vi.mock("@mui/material-nextjs/v16-appRouter", () => ({
  AppRouterCacheProvider: ({
    children,
    options,
  }: AppRouterCacheProviderProps): ReactNode => (
    <div data-nonce={options?.nonce}>{children}</div>
  ),
}));

import {
  resolveRuntimeCspNonce,
  StyleCacheProvider,
} from "./style-cache-provider";

const nonceProbeSelector = "[data-qbit-nonce-test]";

function appendNonceProbe(tagName: "script" | "style", nonce: string): void {
  const element = document.createElement(tagName);
  element.dataset.qbitNonceTest = "true";
  element.nonce = nonce;
  document.head.append(element);
}

afterEach(() => {
  document.querySelectorAll(nonceProbeSelector).forEach((element) => {
    element.remove();
  });
});

describe("resolveRuntimeCspNonce", () => {
  it("prefers a style nonce", () => {
    appendNonceProbe("style", "  style-nonce  ");
    appendNonceProbe("script", "script-nonce");

    expect(resolveRuntimeCspNonce()).toBe("style-nonce");
  });

  it("falls back to a script nonce", () => {
    appendNonceProbe("script", " script-nonce ");

    expect(resolveRuntimeCspNonce()).toBe("script-nonce");
  });

  it("returns undefined when no nonce elements exist", () => {
    expect(resolveRuntimeCspNonce()).toBeUndefined();
  });
});

describe("StyleCacheProvider", () => {
  it("propagates the runtime nonce to the cache provider", () => {
    appendNonceProbe("style", "runtime-nonce");

    render(
      <StyleCacheProvider>
        <span>child</span>
      </StyleCacheProvider>,
    );

    expect(screen.getByText("child").parentElement).toHaveAttribute(
      "data-nonce",
      "runtime-nonce",
    );
  });
});
