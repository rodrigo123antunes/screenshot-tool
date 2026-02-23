import { describe, expect, it } from "vitest";

import type {
  CaptureMode,
  CapturePipelineState,
  CaptureResult,
  FreezeReadyPayload,
  Region,
  StructuredError,
} from "../types";

describe("Capture types contracts", () => {
  it("mantem contrato de StructuredError serializavel para IPC", () => {
    const error: StructuredError = {
      code: "INVALID_STATE",
      message: "Operacao invalida para o estado atual",
      context: "state=Idle",
    };

    expect(error.code).toBe("INVALID_STATE");
    expect(error.context).toContain("state=");
  });

  it("mantem campos obrigatorios para payload de freeze-ready", () => {
    const payload: FreezeReadyPayload = {
      temp_path: "/tmp/sample.png",
      monitor: {
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
        scale_factor: 1,
      },
    };

    expect(payload.monitor.scale_factor).toBe(1);
    expect(payload.temp_path).toContain(".png");
  });

  it("mantem consistencia entre Region e CaptureResult", () => {
    const region: Region = {
      x: 10,
      y: 12,
      width: 320,
      height: 200,
    };
    const result: CaptureResult = {
      path: "/home/user/.local/share/screenshot-tool/captures/2026-02-23_14-35-22_region.png",
      width: 320,
      height: 200,
      file_size: 245760,
      is_black_warning: false,
    };

    expect(region.width).toBeGreaterThan(0);
    expect(region.height).toBeGreaterThan(0);
    expect(result.path).toContain(".png");
    expect(result.width).toBe(320);
    expect(result.height).toBe(200);
    expect(result.file_size).toBeGreaterThan(0);
    expect(result.is_black_warning).toBe(false);
  });

  it("mantem CaptureResult com campo is_black_warning como boolean", () => {
    const warningResult: CaptureResult = {
      path: "/tmp/test_black.png",
      width: 1920,
      height: 1080,
      file_size: 512000,
      is_black_warning: true,
    };
    expect(warningResult.is_black_warning).toBe(true);
  });

  it("aceita apenas modos de captura canonicos", () => {
    const modes: CaptureMode[] = ["fullscreen", "window", "area"];
    expect(modes).toHaveLength(3);
  });

  it("aceita apenas estados de pipeline canonicos", () => {
    const states: CapturePipelineState[] = [
      "idle",
      "capturing",
      "freeze_ready",
      "selecting",
      "finalizing",
      "complete",
      "failed",
      "cancelled",
    ];

    expect(states).toContain("freeze_ready");
    expect(states).toContain("cancelled");
  });
});
