interface FreezeFrameBackgroundProps {
  imageUrl: string | null | undefined;
  width: number;
  height: number;
}

export function FreezeFrameBackground({ imageUrl, width, height }: FreezeFrameBackgroundProps) {
  if (!imageUrl) return null;

  return (
    <div
      data-testid="freeze-frame-background"
      style={{
        position: "fixed",
        top: 0,
        left: 0,
        width: `${width}px`,
        height: `${height}px`,
        backgroundImage: `url(${imageUrl})`,
        backgroundSize: "cover",
        backgroundPosition: "center",
        backgroundRepeat: "no-repeat",
      }}
    />
  );
}
