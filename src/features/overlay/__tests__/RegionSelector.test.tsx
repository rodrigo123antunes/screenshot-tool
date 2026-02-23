import { render, screen, fireEvent } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";

import { RegionSelector } from "../components/RegionSelector";

import type { MonitorInfo } from "@/features/capture/types";

const mockInvoke = invoke as ReturnType<typeof vi.fn>;

const defaultMonitor: MonitorInfo = {
  x: 0,
  y: 0,
  width: 1920,
  height: 1080,
  scale_factor: 1,
};

const hiDpiMonitor: MonitorInfo = {
  x: 0,
  y: 0,
  width: 2560,
  height: 1440,
  scale_factor: 2,
};

describe("RegionSelector", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockInvoke.mockResolvedValue(undefined);
  });

  it("renderiza o container principal com cursor crosshair", () => {
    render(<RegionSelector monitor={defaultMonitor} />);
    const container = screen.getByTestId("region-selector");
    expect(container).toBeInTheDocument();
    expect(container).toHaveStyle({ cursor: "crosshair" });
  });

  it("não mostra seleção antes do mousedown", () => {
    render(<RegionSelector monitor={defaultMonitor} />);
    expect(screen.queryByTestId("selection-rect")).not.toBeInTheDocument();
    expect(screen.queryByTestId("mask-top")).not.toBeInTheDocument();
  });

  it("mousemove sem mousedown não cria seleção", () => {
    render(<RegionSelector monitor={defaultMonitor} />);
    const container = screen.getByTestId("region-selector");
    fireEvent.mouseMove(container, { clientX: 100, clientY: 100 });
    expect(screen.queryByTestId("selection-rect")).not.toBeInTheDocument();
  });

  it("sequência mousedown + mousemove + mouseup produz seleção visual", () => {
    render(<RegionSelector monitor={defaultMonitor} />);
    const container = screen.getByTestId("region-selector");

    fireEvent.mouseDown(container, { clientX: 100, clientY: 100 });
    fireEvent.mouseMove(container, { clientX: 300, clientY: 250 });
    fireEvent.mouseUp(container, { clientX: 300, clientY: 250 });

    expect(screen.getByTestId("selection-rect")).toBeInTheDocument();
    expect(screen.getByTestId("mask-top")).toBeInTheDocument();
    expect(screen.getByTestId("mask-bottom")).toBeInTheDocument();
    expect(screen.getByTestId("mask-left")).toBeInTheDocument();
    expect(screen.getByTestId("mask-right")).toBeInTheDocument();
  });

  it("exibe dimensões em pixels físicos no label de dimensão durante drag", () => {
    render(<RegionSelector monitor={hiDpiMonitor} />);
    const container = screen.getByTestId("region-selector");

    fireEvent.mouseDown(container, { clientX: 100, clientY: 100 });
    fireEvent.mouseMove(container, { clientX: 200, clientY: 200 });

    const label = screen.getByTestId("dimension-label");
    // 100 CSS pixels * scale_factor 2 = 200 physical pixels
    expect(label).toHaveTextContent("200 x 200");
  });

  it("Escape chama cancel_capture exatamente uma vez", () => {
    render(<RegionSelector monitor={defaultMonitor} />);

    fireEvent.keyDown(window, { key: "Escape" });

    expect(mockInvoke).toHaveBeenCalledTimes(1);
    expect(mockInvoke).toHaveBeenCalledWith("cancel_capture");
  });

  it("Enter com seleção ativa chama finalize_capture com coordenadas ajustadas pelo scale_factor", () => {
    render(<RegionSelector monitor={hiDpiMonitor} />);
    const container = screen.getByTestId("region-selector");

    fireEvent.mouseDown(container, { clientX: 50, clientY: 60 });
    fireEvent.mouseMove(container, { clientX: 150, clientY: 180 });
    fireEvent.mouseUp(container, { clientX: 150, clientY: 180 });

    fireEvent.keyDown(window, { key: "Enter" });

    expect(mockInvoke).toHaveBeenCalledWith("finalize_capture", {
      region: {
        x: 100, // 50 * 2
        y: 120, // 60 * 2
        width: 200, // 100 * 2
        height: 240, // 120 * 2
      },
    });
  });

  it("Enter com seleção scale_factor=1 passa coordenadas corretas", () => {
    render(<RegionSelector monitor={defaultMonitor} />);
    const container = screen.getByTestId("region-selector");

    fireEvent.mouseDown(container, { clientX: 10, clientY: 20 });
    fireEvent.mouseMove(container, { clientX: 310, clientY: 220 });
    fireEvent.mouseUp(container, { clientX: 310, clientY: 220 });

    fireEvent.keyDown(window, { key: "Enter" });

    expect(mockInvoke).toHaveBeenCalledWith("finalize_capture", {
      region: {
        x: 10,
        y: 20,
        width: 300,
        height: 200,
      },
    });
  });

  it("Enter sem seleção ativa (sem drag) NÃO chama finalize_capture", () => {
    render(<RegionSelector monitor={defaultMonitor} />);

    fireEvent.keyDown(window, { key: "Enter" });

    expect(mockInvoke).not.toHaveBeenCalled();
  });

  it("mousedown apenas (sem drag) não aciona finalização", () => {
    render(<RegionSelector monitor={defaultMonitor} />);
    const container = screen.getByTestId("region-selector");

    fireEvent.mouseDown(container, { clientX: 100, clientY: 100 });

    fireEvent.keyDown(window, { key: "Enter" });

    // Sem movimento, width e height são 0, portanto Enter não deve chamar finalize
    expect(mockInvoke).not.toHaveBeenCalled();
  });

  it("seleção invertida (arrastar da direita para esquerda) produz coordenadas corretas", () => {
    render(<RegionSelector monitor={defaultMonitor} />);
    const container = screen.getByTestId("region-selector");

    // Arrasta da direita para a esquerda
    fireEvent.mouseDown(container, { clientX: 300, clientY: 200 });
    fireEvent.mouseMove(container, { clientX: 100, clientY: 100 });
    fireEvent.mouseUp(container, { clientX: 100, clientY: 100 });

    fireEvent.keyDown(window, { key: "Enter" });

    expect(mockInvoke).toHaveBeenCalledWith("finalize_capture", {
      region: {
        x: 100,
        y: 100,
        width: 200,
        height: 100,
      },
    });
  });

  it("máscara escura é renderizada fora da seleção durante drag ativo", () => {
    render(<RegionSelector monitor={defaultMonitor} />);
    const container = screen.getByTestId("region-selector");

    fireEvent.mouseDown(container, { clientX: 100, clientY: 100 });
    fireEvent.mouseMove(container, { clientX: 300, clientY: 300 });

    // Máscara deve estar presente durante o drag
    expect(screen.getByTestId("mask-top")).toBeInTheDocument();
    expect(screen.getByTestId("mask-bottom")).toBeInTheDocument();
    expect(screen.getByTestId("mask-left")).toBeInTheDocument();
    expect(screen.getByTestId("mask-right")).toBeInTheDocument();

    // Verificar que as máscaras têm a cor rgba correta
    const maskTop = screen.getByTestId("mask-top");
    expect(maskTop).toHaveStyle({ backgroundColor: "rgba(0,0,0,0.5)" });
  });
});
