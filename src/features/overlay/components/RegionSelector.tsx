import { useCallback, useEffect, useRef, useState } from "react";

import { cancelCapture, finalizeCapture } from "@/features/capture/captureCommands";

import type { MonitorInfo, Region } from "@/features/capture/types";

interface SelectionRect {
  startX: number;
  startY: number;
  endX: number;
  endY: number;
}

interface RegionSelectorProps {
  monitor: MonitorInfo;
}

function rectToRegion(rect: SelectionRect, scaleFactor: number): Region {
  const x = Math.round(Math.min(rect.startX, rect.endX) * scaleFactor);
  const y = Math.round(Math.min(rect.startY, rect.endY) * scaleFactor);
  const width = Math.round(Math.abs(rect.endX - rect.startX) * scaleFactor);
  const height = Math.round(Math.abs(rect.endY - rect.startY) * scaleFactor);
  return { x, y, width, height };
}

export function RegionSelector({ monitor }: RegionSelectorProps) {
  const [isDragging, setIsDragging] = useState(false);
  const [selection, setSelection] = useState<SelectionRect | null>(null);

  const isDraggingRef = useRef(false);
  const selectionRef = useRef<SelectionRect | null>(null);

  // Keep refs in sync with state for use in event handlers
  useEffect(() => {
    isDraggingRef.current = isDragging;
  });
  useEffect(() => {
    selectionRef.current = selection;
  });

  const handleMouseDown = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
    e.preventDefault();
    setIsDragging(true);
    const rect: SelectionRect = {
      startX: e.clientX,
      startY: e.clientY,
      endX: e.clientX,
      endY: e.clientY,
    };
    setSelection(rect);
  }, []);

  const handleMouseMove = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
    if (!isDraggingRef.current) return;
    setSelection((prev) => {
      if (!prev) return prev;
      return { ...prev, endX: e.clientX, endY: e.clientY };
    });
  }, []);

  const handleMouseUp = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
    if (!isDraggingRef.current) return;
    e.preventDefault();
    setIsDragging(false);
    setSelection((prev) => {
      if (!prev) return prev;
      return { ...prev, endX: e.clientX, endY: e.clientY };
    });
  }, []);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        cancelCapture().catch(console.error);
      } else if (e.key === "Enter") {
        const currentSelection = selectionRef.current;
        if (
          currentSelection &&
          Math.abs(currentSelection.endX - currentSelection.startX) > 0 &&
          Math.abs(currentSelection.endY - currentSelection.startY) > 0
        ) {
          const region = rectToRegion(currentSelection, monitor.scale_factor);
          finalizeCapture(region).catch(console.error);
        }
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => {
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [monitor.scale_factor]);

  const selX = selection ? Math.min(selection.startX, selection.endX) : 0;
  const selY = selection ? Math.min(selection.startY, selection.endY) : 0;
  const selW = selection ? Math.abs(selection.endX - selection.startX) : 0;
  const selH = selection ? Math.abs(selection.endY - selection.startY) : 0;

  return (
    <div
      data-testid="region-selector"
      style={{
        position: "fixed",
        top: 0,
        left: 0,
        width: `${monitor.width}px`,
        height: `${monitor.height}px`,
        cursor: "crosshair",
      }}
      onMouseDown={handleMouseDown}
      onMouseMove={handleMouseMove}
      onMouseUp={handleMouseUp}
    >
      {selection && selW > 0 && selH > 0 && (
        <>
          {/* Dark mask - top */}
          <div
            data-testid="mask-top"
            style={{
              position: "absolute",
              top: 0,
              left: 0,
              width: "100%",
              height: `${selY}px`,
              backgroundColor: "rgba(0,0,0,0.5)",
            }}
          />
          {/* Dark mask - bottom */}
          <div
            data-testid="mask-bottom"
            style={{
              position: "absolute",
              top: `${selY + selH}px`,
              left: 0,
              width: "100%",
              bottom: 0,
              backgroundColor: "rgba(0,0,0,0.5)",
            }}
          />
          {/* Dark mask - left */}
          <div
            data-testid="mask-left"
            style={{
              position: "absolute",
              top: `${selY}px`,
              left: 0,
              width: `${selX}px`,
              height: `${selH}px`,
              backgroundColor: "rgba(0,0,0,0.5)",
            }}
          />
          {/* Dark mask - right */}
          <div
            data-testid="mask-right"
            style={{
              position: "absolute",
              top: `${selY}px`,
              left: `${selX + selW}px`,
              right: 0,
              height: `${selH}px`,
              backgroundColor: "rgba(0,0,0,0.5)",
            }}
          />
          {/* Selection border */}
          <div
            data-testid="selection-rect"
            style={{
              position: "absolute",
              top: `${selY}px`,
              left: `${selX}px`,
              width: `${selW}px`,
              height: `${selH}px`,
              border: "2px solid white",
              boxSizing: "border-box",
            }}
          />
          {/* Dimension label */}
          <div
            data-testid="dimension-label"
            style={{
              position: "absolute",
              top: `${selY + selH + 4}px`,
              left: `${selX}px`,
              color: "white",
              fontSize: "12px",
              backgroundColor: "rgba(0,0,0,0.7)",
              padding: "2px 6px",
              borderRadius: "2px",
              pointerEvents: "none",
            }}
          >
            {Math.round(selW * monitor.scale_factor)} x {Math.round(selH * monitor.scale_factor)}
          </div>
        </>
      )}
    </div>
  );
}
