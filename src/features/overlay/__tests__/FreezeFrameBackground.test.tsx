import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { FreezeFrameBackground } from "../components/FreezeFrameBackground";

describe("FreezeFrameBackground", () => {
  it("renderiza um div com background-image quando imageUrl é fornecido", () => {
    render(
      <FreezeFrameBackground
        imageUrl="asset://localhost/tmp/screenshot.png"
        width={1920}
        height={1080}
      />,
    );

    const el = screen.getByTestId("freeze-frame-background");
    expect(el).toBeInTheDocument();
    expect(el).toHaveStyle({
      backgroundImage: "url(asset://localhost/tmp/screenshot.png)",
    });
  });

  it("aplica estilização full-screen com width e height do monitor", () => {
    render(
      <FreezeFrameBackground
        imageUrl="asset://localhost/tmp/screenshot.png"
        width={2560}
        height={1440}
      />,
    );

    const el = screen.getByTestId("freeze-frame-background");
    expect(el).toHaveStyle({
      position: "fixed",
      top: "0px",
      left: "0px",
      width: "2560px",
      height: "1440px",
    });
  });

  it("não renderiza nada quando imageUrl é null", () => {
    render(<FreezeFrameBackground imageUrl={null} width={1920} height={1080} />);
    expect(screen.queryByTestId("freeze-frame-background")).not.toBeInTheDocument();
  });

  it("não renderiza nada quando imageUrl é undefined", () => {
    render(<FreezeFrameBackground imageUrl={undefined} width={1920} height={1080} />);
    expect(screen.queryByTestId("freeze-frame-background")).not.toBeInTheDocument();
  });

  it("aplica backgroundSize cover e backgroundPosition center", () => {
    render(
      <FreezeFrameBackground
        imageUrl="asset://localhost/tmp/test.png"
        width={1920}
        height={1080}
      />,
    );

    const el = screen.getByTestId("freeze-frame-background");
    expect(el).toHaveStyle({
      backgroundSize: "cover",
      backgroundPosition: "center",
    });
  });
});
