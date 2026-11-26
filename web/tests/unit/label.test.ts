import { describe, expect, it } from "vitest";
import { wrapLabel } from "../../src/label.ts";

describe("wrapLabel", () => {
  it("keeps a short label on one line", () => {
    expect(wrapLabel("Check stock", 17, 2)).toEqual(["Check stock"]);
  });
  it("breaks a longer label at a space instead of cutting it", () => {
    expect(wrapLabel("Order from supplier", 17, 2)).toEqual(["Order from", "supplier"]);
  });
  it("fills a line up to exactly the width", () => {
    expect(wrapLabel("abcd efgh ij", 9, 2)).toEqual(["abcd efgh", "ij"]);
  });
  it("marks text that does not fit in the lines allowed", () => {
    expect(wrapLabel("one two three four five six", 9, 2)).toEqual(["one two", "three…"]);
    expect(wrapLabel("Order from supplier", 17, 1)).toEqual(["Order from…"]);
  });
  it("cuts a single word that is wider than a line", () => {
    expect(wrapLabel("incomprehensibilities now", 10, 2)).toEqual(["incompreh…", "now"]);
  });
  it("counts an emoji as one character, so three of them fit a width of four", () => {
    expect(wrapLabel("📦📦📦", 4, 1)).toEqual(["📦📦📦"]);
  });
  it("never cuts an emoji in half", () => {
    expect(wrapLabel("📦📦📦📦📦📦", 4, 1)).toEqual(["📦📦📦…"]);
  });
  it("returns no lines for blank text", () => {
    expect(wrapLabel("   ", 17, 2)).toEqual([]);
  });
});
