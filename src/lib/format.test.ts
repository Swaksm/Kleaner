import { expect, test } from "vitest";
import { formatBytes } from "./format";

test("unités binaires FR et EN", () => {
  expect(formatBytes(0, "fr")).toBe("0 o");
  expect(formatBytes(1023, "en")).toBe("1023 B");
  expect(formatBytes(1024, "en")).toBe("1.0 KB");
  expect(formatBytes(1536 * 1024 * 1024, "en")).toBe("1.5 GB");
  expect(formatBytes(1536 * 1024 * 1024, "fr")).toBe("1,5 Go");
});
