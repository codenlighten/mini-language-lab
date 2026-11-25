import { describe, expect, it } from "vitest";
import { buildSegments, displayRange } from "../../src/highlight.ts";
import type { Diagnostic, Token } from "../../src/types.ts";

const diag = (start: number, end: number, severity: "error" | "warning" = "error"): Diagnostic => ({
  code: "E000", severity, message: "m", hint: null, start, end, line: 0, column: 0, endLine: 0, endColumn: 0,
});
const tok = (kind: Token["kind"], start: number, end: number): Token => ({ kind, start, end });
const joined = (s: { text: string }[]) => s.map((x) => x.text).join("");

describe("buildSegments", () => {
  it("always joins back into exactly the original text", () => {
    const text = "task a -> b // note\n";
    const segs = buildSegments(text, [tok("keyword", 0, 4), tok("name", 5, 6), tok("punct", 7, 9), tok("name", 10, 11), tok("comment", 12, 19)], [diag(5, 6)]);
    expect(joined(segs)).toBe(text);
  });

  it("segments are contiguous, ordered and non-empty", () => {
    const text = "abc def ghi";
    const segs = buildSegments(text, [tok("name", 0, 3), tok("name", 8, 11)], [diag(2, 9)]);
    let at = 0;
    for (const s of segs) {
      expect(s.start).toBe(at);
      expect(s.end).toBeGreaterThan(s.start);
      at = s.end;
    }
    expect(at).toBe(text.length);
  });

  it("gaps between tokens (spaces) get no kind", () => {
    const segs = buildSegments("a b", [tok("name", 0, 1), tok("name", 2, 3)], []);
    expect(segs.map((s) => s.kind)).toEqual(["name", null, "name"]);
  });

  it("merges neighbours that look the same", () => {
    const segs = buildSegments("aaaa", [tok("name", 0, 2), tok("name", 2, 4)], []);
    expect(segs).toHaveLength(1);
  });

  it("an error underline wins over a warning where they overlap, and splits the token", () => {
    const segs = buildSegments("abcdef", [tok("name", 0, 6)], [diag(0, 4, "warning"), diag(2, 6, "error")]);
    expect(segs.map((s) => [s.text, s.severity])).toEqual([["ab", "warning"], ["cdef", "error"]]);
  });

  it("works with no tokens and no diagnostics, and with empty text", () => {
    expect(buildSegments("plain", [], [])).toEqual([{ text: "plain", start: 0, end: 5, kind: null, severity: null }]);
    expect(buildSegments("", [], [])).toEqual([]);
  });

  it("clamps out-of-range offsets instead of throwing", () => {
    const segs = buildSegments("abc", [tok("name", -5, 99)], [diag(-3, 50)]);
    expect(joined(segs)).toBe("abc");
  });

  it("never splits an emoji (a surrogate pair) when a diagnostic points at it", () => {
    const text = "a😀b";
    const segs = buildSegments(text, [], [diag(1, 1)]);
    expect(joined(segs)).toBe(text);
    expect(segs.some((s) => s.text === "😀" && s.severity === "error")).toBe(true);
    for (const s of segs) expect(s.text).not.toMatch(/^[\udc00-\udfff]|[\ud800-\udbff]$/);
  });
});

describe("displayRange", () => {
  it("keeps a normal range as it is", () => {
    expect(displayRange({ start: 2, end: 5 }, "0123456789")).toEqual([2, 5]);
  });
  it("widens a zero-width range by one character", () => {
    expect(displayRange({ start: 3, end: 3 }, "0123456789")).toEqual([3, 4]);
  });
  it("widens a zero-width range at the very end backwards", () => {
    expect(displayRange({ start: 10, end: 10 }, "0123456789")).toEqual([9, 10]);
  });
  it("widens over a whole emoji, forwards and backwards", () => {
    expect(displayRange({ start: 1, end: 1 }, "a😀b")).toEqual([1, 3]);
    expect(displayRange({ start: 3, end: 3 }, "a😀")).toEqual([1, 3]);
  });
  it("leaves an empty text alone", () => {
    expect(displayRange({ start: 0, end: 0 }, "")).toEqual([0, 0]);
  });
});
