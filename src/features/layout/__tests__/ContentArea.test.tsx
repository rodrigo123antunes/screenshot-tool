import { render, screen } from "@testing-library/react";
import { describe, it, expect } from "vitest";
import { ContentArea } from "../components/ContentArea";

describe("ContentArea", () => {
  it("renders without crashing", () => {
    render(<ContentArea />);
    expect(document.querySelector("main")).toBeInTheDocument();
  });

  it("renders children passed to it", () => {
    render(
      <ContentArea>
        <p data-testid="child">Hello Content</p>
      </ContentArea>,
    );

    expect(screen.getByTestId("child")).toBeInTheDocument();
    expect(screen.getByTestId("child")).toHaveTextContent("Hello Content");
  });

  it("container is a flexible element that grows to fill available space", () => {
    render(<ContentArea />);
    const main = document.querySelector("main");
    expect(main).toHaveClass("flex-1");
  });
});
