import { convertFileSrc } from "@tauri-apps/api/core";
import { useState } from "react";

import { useCaptureEvents } from "@/features/capture/useCaptureEvents";

import type { FreezeReadyPayload } from "@/features/capture/types";

import { FreezeFrameBackground } from "./FreezeFrameBackground";
import { RegionSelector } from "./RegionSelector";

export function OverlayView() {
  const [freezePayload, setFreezePayload] = useState<FreezeReadyPayload | null>(null);
  const [imageUrl, setImageUrl] = useState<string | null>(null);
  const [isFlashing, setIsFlashing] = useState(false);

  useCaptureEvents({
    onFreezeReady: (payload) => {
      const url = convertFileSrc(payload.temp_path);
      setFreezePayload(payload);
      setImageUrl(url);
    },
    onComplete: () => {
      setIsFlashing(true);
      setTimeout(() => {
        setIsFlashing(false);
      }, 200);
    },
    onCancelled: () => {
      setFreezePayload(null);
      setImageUrl(null);
    },
  });

  return (
    <div
      data-testid="overlay-view"
      style={{
        position: "fixed",
        top: 0,
        left: 0,
        width: "100vw",
        height: "100vh",
        overflow: "hidden",
      }}
    >
      {imageUrl && freezePayload && (
        <FreezeFrameBackground
          imageUrl={imageUrl}
          width={freezePayload.monitor.width}
          height={freezePayload.monitor.height}
        />
      )}
      {freezePayload && <RegionSelector monitor={freezePayload.monitor} />}
      {isFlashing && (
        <div
          data-testid="flash-effect"
          style={{
            position: "fixed",
            top: 0,
            left: 0,
            width: "100%",
            height: "100%",
            backgroundColor: "white",
            opacity: 0.8,
            pointerEvents: "none",
          }}
        />
      )}
    </div>
  );
}
