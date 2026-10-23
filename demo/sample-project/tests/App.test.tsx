import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { Header } from "../web/src/components/Header";

describe("Header", () => {
  it("shows the total number of jobs", () => {
    render(<Header connected total={1234} />);
    expect(screen.getByText("1,234 jobs")).toBeTruthy();
  });
});
