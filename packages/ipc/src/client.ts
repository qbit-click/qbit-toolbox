import { invoke } from "@tauri-apps/api/core";

import type { CoreStatusDto } from "../generated";

/**
 * The only browser-facing boundary for commands implemented by the Tauri host.
 * Command strings intentionally stay here so UI packages never depend on Tauri.
 */
export interface IpcClient {
  getCoreStatus(): Promise<CoreStatusDto>;
}

export function createIpcClient(
  invokeCommand: <T>(
    command: string,
    args?: Record<string, unknown>,
  ) => Promise<T> = invoke,
): IpcClient {
  return {
    getCoreStatus: () => invokeCommand<CoreStatusDto>("get_core_status"),
  };
}

export const ipc = createIpcClient();
