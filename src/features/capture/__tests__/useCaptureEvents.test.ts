import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(),
}));

import { listen } from "@tauri-apps/api/event";

import { useCaptureEvents } from "../useCaptureEvents";

import type { CaptureResult, FreezeReadyPayload, StructuredError } from "../types";

type EventHandler = (event: { payload: unknown }) => void;
type ListenMock = ReturnType<typeof vi.fn>;

const mockListen = listen as ListenMock;

describe("useCaptureEvents", () => {
  const mockUnlisten = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    mockListen.mockResolvedValue(mockUnlisten);
  });

  it("registra listeners para todos os 4 eventos", async () => {
    renderHook(() => useCaptureEvents({}));

    await waitFor(() => {
      expect(mockListen).toHaveBeenCalledTimes(4);
    });

    expect(mockListen).toHaveBeenCalledWith("capture:freeze-ready", expect.any(Function));
    expect(mockListen).toHaveBeenCalledWith("capture:complete", expect.any(Function));
    expect(mockListen).toHaveBeenCalledWith("capture:error", expect.any(Function));
    expect(mockListen).toHaveBeenCalledWith("capture:cancelled", expect.any(Function));
  });

  it("chama onFreezeReady com FreezeReadyPayload quando evento capture:freeze-ready é emitido", async () => {
    const onFreezeReady = vi.fn();
    const freezePayload: FreezeReadyPayload = {
      temp_path: "/tmp/screenshot.png",
      monitor: { x: 0, y: 0, width: 1920, height: 1080, scale_factor: 1 },
    };

    const capturedHandlers: Record<string, EventHandler> = {};
    mockListen.mockImplementation(async (event: string, handler: EventHandler) => {
      capturedHandlers[event] = handler;
      return mockUnlisten;
    });

    renderHook(() => useCaptureEvents({ onFreezeReady }));

    await waitFor(() => {
      expect(mockListen).toHaveBeenCalledWith("capture:freeze-ready", expect.any(Function));
    });

    act(() => {
      capturedHandlers["capture:freeze-ready"]({ payload: freezePayload });
    });

    expect(onFreezeReady).toHaveBeenCalledWith(freezePayload);
  });

  it("chama onComplete com CaptureResult quando evento capture:complete é emitido", async () => {
    const onComplete = vi.fn();
    const captureResult: CaptureResult = {
      file_path: "/home/user/Screenshots/screenshot.png",
      clipboard_success: true,
    };

    const capturedHandlers: Record<string, EventHandler> = {};
    mockListen.mockImplementation(async (event: string, handler: EventHandler) => {
      capturedHandlers[event] = handler;
      return mockUnlisten;
    });

    renderHook(() => useCaptureEvents({ onComplete }));

    await waitFor(() => {
      expect(mockListen).toHaveBeenCalledWith("capture:complete", expect.any(Function));
    });

    act(() => {
      capturedHandlers["capture:complete"]({ payload: captureResult });
    });

    expect(onComplete).toHaveBeenCalledWith(captureResult);
  });

  it("chama onError com StructuredError quando evento capture:error é emitido", async () => {
    const onError = vi.fn();
    const errorPayload: StructuredError = {
      code: "CAPTURE_FAILED",
      message: "Falha ao capturar a tela",
      context: "state=Capturing",
    };

    const capturedHandlers: Record<string, EventHandler> = {};
    mockListen.mockImplementation(async (event: string, handler: EventHandler) => {
      capturedHandlers[event] = handler;
      return mockUnlisten;
    });

    renderHook(() => useCaptureEvents({ onError }));

    await waitFor(() => {
      expect(mockListen).toHaveBeenCalledWith("capture:error", expect.any(Function));
    });

    act(() => {
      capturedHandlers["capture:error"]({ payload: errorPayload });
    });

    expect(onError).toHaveBeenCalledWith(errorPayload);
  });

  it("chama onCancelled quando evento capture:cancelled é emitido", async () => {
    const onCancelled = vi.fn();

    const capturedHandlers: Record<string, EventHandler> = {};
    mockListen.mockImplementation(async (event: string, handler: EventHandler) => {
      capturedHandlers[event] = handler;
      return mockUnlisten;
    });

    renderHook(() => useCaptureEvents({ onCancelled }));

    await waitFor(() => {
      expect(mockListen).toHaveBeenCalledWith("capture:cancelled", expect.any(Function));
    });

    act(() => {
      capturedHandlers["capture:cancelled"]({ payload: undefined });
    });

    expect(onCancelled).toHaveBeenCalledTimes(1);
  });

  it("limpa todos os listeners (unlisten) quando componente é desmontado", async () => {
    const { unmount } = renderHook(() => useCaptureEvents({}));

    await waitFor(() => {
      expect(mockListen).toHaveBeenCalledTimes(4);
    });

    unmount();

    await waitFor(() => {
      expect(mockUnlisten).toHaveBeenCalledTimes(4);
    });
  });

  it("não re-registra listeners em re-renders", async () => {
    const { rerender } = renderHook(() => useCaptureEvents({}));

    await waitFor(() => {
      expect(mockListen).toHaveBeenCalledTimes(4);
    });

    rerender();
    rerender();

    // Número de chamadas não deve aumentar após re-renders (dependências vazias no useEffect)
    expect(mockListen).toHaveBeenCalledTimes(4);
  });

  it("limpa listeners imediatamente quando componente desmonta antes do setup terminar", async () => {
    const earlyUnlisten = vi.fn();
    const resolvers: Array<(fn: () => void) => void> = [];

    // Cria promises que não resolvem até sinalizar manualmente
    mockListen.mockImplementation(
      () =>
        new Promise<() => void>((resolve) => {
          resolvers.push(resolve);
        }),
    );

    const { unmount } = renderHook(() => useCaptureEvents({}));

    // Aguarda as 4 chamadas ao listen antes de desmontar
    await waitFor(() => expect(mockListen).toHaveBeenCalledTimes(4));

    // Desmonta antes das promises resolverem
    unmount();

    // Resolve todas as promises após o desmonte — deve acionar cleanup imediato
    act(() => {
      resolvers.forEach((resolve) => resolve(earlyUnlisten));
    });

    // earlyUnlisten deve ser chamado para cada listener pelo caminho de cleanup early
    await waitFor(() => expect(earlyUnlisten).toHaveBeenCalledTimes(4));
  });
});
