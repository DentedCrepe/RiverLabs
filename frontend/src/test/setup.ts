import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/react";
import { afterEach, vi } from "vitest";

afterEach(() => {
  cleanup();
});

// next/navigation is provided by the Next runtime; tests control the pathname explicitly.
vi.mock("next/navigation", () => ({
  usePathname: vi.fn(() => "/"),
}));
