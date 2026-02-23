import { act, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  convertFileSrc: vi.fn((path: string) => `asset://localhost${path}`),
}));

import { listen } from "@tauri-apps/api/event";
import { convertFileSrc } from "@tauri-apps/api/core";

import { OverlayView } from "../components/OverlayView";

import type { CaptureResult, FreezeReadyPayload } from "@/features/capture/types";

type EventHandler = (event: { payload: unknown }) => void;
type ListenMock = ReturnType<typeof vi.fn>;

const mockListen = listen as ListenMock;
const mockConvertFileSrc = convertFileSrc as ReturnType<typeof vi.fn>;

describe("OverlayView", () => {
  const mockUnlisten = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    mockListen.mockResolvedValue(mockUnlisten);
  });

  it("renderiza o container overlay-view", () => {
    render(<OverlayView />);
    expect(screen.getByTestId("overlay-view")).toBeInTheDocument();
  });

  it("não renderiza FreezeFrameBackground antes de receber capture:freeze-ready", () => {
    render(<OverlayView />);
    expect(screen.queryByTestId("freeze-frame-background")).not.toBeInTheDocument();
  });

  it("não renderiza RegionSelector antes de receber capture:freeze-ready", () => {
    render(<OverlayView />);
    expect(screen.queryByTestId("region-selector")).not.toBeInTheDocument();
  });

  it("renderiza FreezeFrameBackground com URL correto após capture:freeze-ready", async () => {
    const freezePayload: FreezeReadyPayload = {
      temp_path: "/tmp/screenshot.png",
      monitor: { x: 0, y: 0, width: 1920, height: 1080, scale_factor: 1 },
    };

    const capturedHandlers: Record<string, EventHandler> = {};
    mockListen.mockImplementation(async (event: string, handler: EventHandler) => {
      capturedHandlers[event] = handler;
      return mockUnlisten;
    });

    mockConvertFileSrc.mockReturnValue("asset://localhost/tmp/screenshot.png");

    render(<OverlayView />);

    await waitFor(() => {
      expect(mockListen).toHaveBeenCalledWith("capture:freeze-ready", expect.any(Function));
    });

    act(() => {
      capturedHandlers["capture:freeze-ready"]({ payload: freezePayload });
    });

    await waitFor(() => {
      expect(screen.getByTestId("freeze-frame-background")).toBeInTheDocument();
    });

    const bg = screen.getByTestId("freeze-frame-background");
    expect(bg).toHaveStyle({
      backgroundImage: "url(asset://localhost/tmp/screenshot.png)",
    });
  });

  it("renderiza RegionSelector após receber capture:freeze-ready", async () => {
    const freezePayload: FreezeReadyPayload = {
      temp_path: "/tmp/screenshot.png",
      monitor: { x: 0, y: 0, width: 1920, height: 1080, scale_factor: 1 },
    };

    const capturedHandlers: Record<string, EventHandler> = {};
    mockListen.mockImplementation(async (event: string, handler: EventHandler) => {
      capturedHandlers[event] = handler;
      return mockUnlisten;
    });

    render(<OverlayView />);

    await waitFor(() => {
      expect(mockListen).toHaveBeenCalledWith("capture:freeze-ready", expect.any(Function));
    });

    act(() => {
      capturedHandlers["capture:freeze-ready"]({ payload: freezePayload });
    });

    await waitFor(() => {
      expect(screen.getByTestId("region-selector")).toBeInTheDocument();
    });
  });

  it("chama convertFileSrc com temp_path correto", async () => {
    const freezePayload: FreezeReadyPayload = {
      temp_path: "/tmp/abc-def-123.png",
      monitor: { x: 0, y: 0, width: 1920, height: 1080, scale_factor: 1 },
    };

    const capturedHandlers: Record<string, EventHandler> = {};
    mockListen.mockImplementation(async (event: string, handler: EventHandler) => {
      capturedHandlers[event] = handler;
      return mockUnlisten;
    });

    render(<OverlayView />);

    await waitFor(() => {
      expect(mockListen).toHaveBeenCalledWith("capture:freeze-ready", expect.any(Function));
    });

    act(() => {
      capturedHandlers["capture:freeze-ready"]({ payload: freezePayload });
    });

    expect(mockConvertFileSrc).toHaveBeenCalledWith("/tmp/abc-def-123.png");
  });

  it("registra listeners para todos os 4 eventos de captura", async () => {
    render(<OverlayView />);

    await waitFor(() => {
      expect(mockListen).toHaveBeenCalledTimes(4);
    });

    expect(mockListen).toHaveBeenCalledWith("capture:freeze-ready", expect.any(Function));
    expect(mockListen).toHaveBeenCalledWith("capture:complete", expect.any(Function));
    expect(mockListen).toHaveBeenCalledWith("capture:error", expect.any(Function));
    expect(mockListen).toHaveBeenCalledWith("capture:cancelled", expect.any(Function));
  });

  it("limpa listeners (unlisten) no desmonte", async () => {
    const { unmount } = render(<OverlayView />);

    await waitFor(() => {
      expect(mockListen).toHaveBeenCalledTimes(4);
    });

    unmount();

    await waitFor(() => {
      expect(mockUnlisten).toHaveBeenCalledTimes(4);
    });
  });

  it("evento capture:complete aciona efeito de flash", async () => {
    const completeResult: CaptureResult = {
      file_path: "/home/user/Screenshots/screenshot.png",
      clipboard_success: true,
    };

    const capturedHandlers: Record<string, EventHandler> = {};
    mockListen.mockImplementation(async (event: string, handler: EventHandler) => {
      capturedHandlers[event] = handler;
      return mockUnlisten;
    });

    render(<OverlayView />);

    await waitFor(() => {
      expect(mockListen).toHaveBeenCalledWith("capture:complete", expect.any(Function));
    });

    act(() => {
      capturedHandlers["capture:complete"]({ payload: completeResult });
    });

    await waitFor(() => {
      expect(screen.getByTestId("flash-effect")).toBeInTheDocument();
    });
  });

  it("evento capture:cancelled reseta o estado (esconde RegionSelector e FreezeFrameBackground)", async () => {
    const freezePayload: FreezeReadyPayload = {
      temp_path: "/tmp/screenshot.png",
      monitor: { x: 0, y: 0, width: 1920, height: 1080, scale_factor: 1 },
    };

    const capturedHandlers: Record<string, EventHandler> = {};
    mockListen.mockImplementation(async (event: string, handler: EventHandler) => {
      capturedHandlers[event] = handler;
      return mockUnlisten;
    });

    render(<OverlayView />);

    await waitFor(() => {
      expect(mockListen).toHaveBeenCalledWith("capture:freeze-ready", expect.any(Function));
    });

    // Primeiro aciona o freeze-ready para mostrar os componentes
    act(() => {
      capturedHandlers["capture:freeze-ready"]({ payload: freezePayload });
    });

    await waitFor(() => {
      expect(screen.getByTestId("freeze-frame-background")).toBeInTheDocument();
    });

    // Depois aciona capture:cancelled para resetar
    act(() => {
      capturedHandlers["capture:cancelled"]({ payload: undefined });
    });

    await waitFor(() => {
      expect(screen.queryByTestId("freeze-frame-background")).not.toBeInTheDocument();
      expect(screen.queryByTestId("region-selector")).not.toBeInTheDocument();
    });
  });
});
