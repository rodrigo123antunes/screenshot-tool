import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";

import { cancelCapture, finalizeCapture, startCapture } from "../captureCommands";

import type { CaptureResult, Region, StructuredError } from "../types";

const mockInvoke = invoke as ReturnType<typeof vi.fn>;

describe("captureCommands", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("startCapture", () => {
    it("chama invoke com mode 'area' e retorna void em sucesso", async () => {
      mockInvoke.mockResolvedValueOnce(undefined);

      const result = await startCapture("area");

      expect(mockInvoke).toHaveBeenCalledWith("start_capture", { mode: "area" });
      expect(result).toBeUndefined();
    });

    it("chama invoke com mode 'fullscreen'", async () => {
      mockInvoke.mockResolvedValueOnce(undefined);

      await startCapture("fullscreen");

      expect(mockInvoke).toHaveBeenCalledWith("start_capture", { mode: "fullscreen" });
    });

    it("chama invoke com mode 'window'", async () => {
      mockInvoke.mockResolvedValueOnce(undefined);

      await startCapture("window");

      expect(mockInvoke).toHaveBeenCalledWith("start_capture", { mode: "window" });
    });

    it("propaga erros do invoke como StructuredError", async () => {
      const structuredError: StructuredError = {
        code: "INVALID_STATE",
        message: "Não é possível iniciar captura no estado atual",
        context: "state=Capturing",
      };
      mockInvoke.mockRejectedValueOnce(structuredError);

      await expect(startCapture("area")).rejects.toEqual(structuredError);
    });
  });

  describe("finalizeCapture", () => {
    it("chama invoke com região correta e retorna CaptureResult", async () => {
      const region: Region = { x: 10, y: 20, width: 300, height: 200 };
      const captureResult: CaptureResult = {
        path: "/home/user/.local/share/screenshot-tool/captures/2026-02-23_14-35-22_region.png",
        width: 300,
        height: 200,
        file_size: 245760,
        is_black_warning: false,
      };
      mockInvoke.mockResolvedValueOnce(captureResult);

      const result = await finalizeCapture(region);

      expect(mockInvoke).toHaveBeenCalledWith("finalize_capture", { region });
      expect(result).toEqual(captureResult);
    });

    it("propaga erros do invoke como StructuredError", async () => {
      const region: Region = { x: 0, y: 0, width: 100, height: 100 };
      const structuredError: StructuredError = {
        code: "INVALID_STATE",
        message: "Operação inválida para o estado atual",
      };
      mockInvoke.mockRejectedValueOnce(structuredError);

      await expect(finalizeCapture(region)).rejects.toEqual(structuredError);
    });
  });

  describe("cancelCapture", () => {
    it("chama invoke 'cancel_capture' sem argumentos e retorna void", async () => {
      mockInvoke.mockResolvedValueOnce(undefined);

      const result = await cancelCapture();

      expect(mockInvoke).toHaveBeenCalledWith("cancel_capture");
      expect(result).toBeUndefined();
    });

    it("propaga erros do invoke como StructuredError", async () => {
      const structuredError: StructuredError = {
        code: "INTERNAL_ERROR",
        message: "Mutex envenenado",
      };
      mockInvoke.mockRejectedValueOnce(structuredError);

      await expect(cancelCapture()).rejects.toEqual(structuredError);
    });
  });
});
