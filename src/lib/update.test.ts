import { expect, test } from "vitest";
import { isNewer } from "./update";

test("comparaison de versions, préversions comprises", () => {
  expect(isNewer("0.1.0-rc8", "0.1.0-rc7")).toBe(true);
  expect(isNewer("0.1.0-rc10", "0.1.0-rc9")).toBe(true);
  expect(isNewer("0.1.0", "0.1.0-rc9")).toBe(true);
  expect(isNewer("v0.2.0", "0.1.0")).toBe(true);
  expect(isNewer("0.1.0-rc7", "0.1.0-rc7")).toBe(false);
  expect(isNewer("0.1.0-rc9", "0.1.0")).toBe(false);
  expect(isNewer("0.0.9", "0.1.0")).toBe(false);
  expect(isNewer("1.0.0", "0.1.0-rc18")).toBe(true);
  expect(isNewer("1.1.0", "1.0.0")).toBe(true);
});
