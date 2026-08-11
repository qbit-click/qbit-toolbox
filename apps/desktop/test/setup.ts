import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/react";
import { afterEach } from "vitest";

type SystemColorScheme = "light" | "dark";
type LegacyMediaQueryListListener = NonNullable<
  Parameters<MediaQueryList["addListener"]>[0]
>;

class MediaQueryChangeEvent extends Event implements MediaQueryListEvent {
  constructor(
    readonly media: string,
    readonly matches: boolean,
  ) {
    super("change");
  }
}

class ControllableMediaQueryList extends EventTarget implements MediaQueryList {
  onchange:
    ((this: MediaQueryList, event: MediaQueryListEvent) => unknown) | null =
    null;
  private readonly legacyListeners = new Set<LegacyMediaQueryListListener>();

  constructor(readonly media: string) {
    super();
  }

  get matches(): boolean {
    return (
      this.media === "(prefers-color-scheme: dark)" &&
      systemColorScheme === "dark"
    );
  }

  addListener(listener: LegacyMediaQueryListListener | null): void {
    if (listener !== null) this.legacyListeners.add(listener);
  }

  removeListener(listener: LegacyMediaQueryListListener | null): void {
    if (listener !== null) this.legacyListeners.delete(listener);
  }

  notifyChange(): void {
    const event = new MediaQueryChangeEvent(this.media, this.matches);
    this.onchange?.call(this, event);
    for (const listener of this.legacyListeners) listener.call(this, event);
    this.dispatchEvent(event);
  }
}

const mediaQueryLists = new Map<string, ControllableMediaQueryList>();
let systemColorScheme: SystemColorScheme = "light";

export function setSystemColorScheme(scheme: SystemColorScheme): void {
  if (systemColorScheme === scheme) return;
  systemColorScheme = scheme;
  for (const mediaQueryList of mediaQueryLists.values()) {
    mediaQueryList.notifyChange();
  }
}

Object.defineProperty(window, "matchMedia", {
  configurable: true,
  value: (query: string): MediaQueryList => {
    const existingMediaQueryList = mediaQueryLists.get(query);

    if (existingMediaQueryList) {
      return existingMediaQueryList;
    }

    const mediaQueryList = new ControllableMediaQueryList(query);

    mediaQueryLists.set(query, mediaQueryList);

    return mediaQueryList;
  },
});

afterEach(() => {
  cleanup();
  setSystemColorScheme("light");
});
