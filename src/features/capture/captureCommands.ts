import { invoke } from "@tauri-apps/api/core";

import type { CaptureMode, CaptureResult, Region } from "./types";

export async function startCapture(mode: CaptureMode): Promise<void> {
  return invoke<void>("start_capture", { mode });
}

export async function finalizeCapture(region: Region): Promise<CaptureResult> {
  return invoke<CaptureResult>("finalize_capture", { region });
}

export async function cancelCapture(): Promise<void> {
  return invoke<void>("cancel_capture");
}
