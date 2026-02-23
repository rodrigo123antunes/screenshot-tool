import { listen } from "@tauri-apps/api/event";
import { useEffect, useRef } from "react";

import type { CaptureResult, FreezeReadyPayload, StructuredError } from "./types";

export interface UseCaptureEventsOptions {
  onFreezeReady?: (payload: FreezeReadyPayload) => void;
  onComplete?: (result: CaptureResult) => void;
  onError?: (error: StructuredError) => void;
  onCancelled?: () => void;
}

export function useCaptureEvents({
  onFreezeReady,
  onComplete,
  onError,
  onCancelled,
}: UseCaptureEventsOptions): void {
  const onFreezeReadyRef = useRef(onFreezeReady);
  const onCompleteRef = useRef(onComplete);
  const onErrorRef = useRef(onError);
  const onCancelledRef = useRef(onCancelled);

  // Atualiza refs a cada render sem re-executar o efeito de registro dos listeners.
  // Isso garante que callbacks atualizados sejam usados sem re-registrar listeners.
  useEffect(() => {
    onFreezeReadyRef.current = onFreezeReady;
  });
  useEffect(() => {
    onCompleteRef.current = onComplete;
  });
  useEffect(() => {
    onErrorRef.current = onError;
  });
  useEffect(() => {
    onCancelledRef.current = onCancelled;
  });

  useEffect(() => {
    let mounted = true;
    const unlisteners: Array<() => void> = [];

    const setup = async () => {
      const results = await Promise.all([
        listen<FreezeReadyPayload>("capture:freeze-ready", (event) => {
          onFreezeReadyRef.current?.(event.payload);
        }),
        listen<CaptureResult>("capture:complete", (event) => {
          onCompleteRef.current?.(event.payload);
        }),
        listen<StructuredError>("capture:error", (event) => {
          onErrorRef.current?.(event.payload);
        }),
        listen<void>("capture:cancelled", () => {
          onCancelledRef.current?.();
        }),
      ]);

      if (mounted) {
        unlisteners.push(...results);
      } else {
        // Componente desmontado antes do setup terminar: limpa imediatamente
        results.forEach((unlisten) => unlisten());
      }
    };

    setup().catch(console.error);

    return () => {
      mounted = false;
      unlisteners.forEach((unlisten) => unlisten());
    };
  }, []); // Dependências vazias: registra uma vez, nunca re-registra em re-renders
}
