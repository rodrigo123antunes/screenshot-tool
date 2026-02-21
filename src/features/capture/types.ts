export interface Region {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface MonitorInfo {
  x: number;
  y: number;
  width: number;
  height: number;
  scale_factor: number;
}

export interface CaptureResult {
  file_path: string;
  clipboard_success: boolean;
}

export interface StructuredError {
  code: string;
  message: string;
  context?: string;
}

export interface FreezeReadyPayload {
  temp_path: string;
  monitor: MonitorInfo;
}

export type CaptureMode = "fullscreen" | "window" | "area";

export type CapturePipelineState =
  | "idle"
  | "capturing"
  | "freeze_ready"
  | "selecting"
  | "finalizing"
  | "complete"
  | "failed"
  | "cancelled";
