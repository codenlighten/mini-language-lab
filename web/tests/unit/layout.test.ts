import { describe, expect, it } from "vitest";
import { COLUMN_GAP, layoutProcess } from "../../src/layout.ts";
import type { GraphNode, ProcessModel } from "../../src/types.ts";

const n = (id: string, kind: GraphNode["kind"], depth: number | null): GraphNode => ({ id, kind, label: null, depth });

const ORDER: ProcessModel = {
  name: "Order",
  nodes: [n("start", "start", 0), n("check", "task", 1), n("decide", "gateway", 2), n("ship", "task", 3), n("backorder", "task", 3), n("done", "end", 4)],
  edges: [
    { from: "start", to: "check", label: null },
    { from: "check", to: "decide", label: null },
    { from: "decide", to: "ship", label: "yes" },
    { from: "decide", to: "backorder", label: "no" },
    { from: "ship", to: "done", label: null },
    { from: "backorder", to: "check", label: null },
  ],
};

describe("layoutProcess", () => {
  const l = layoutProcess(ORDER);

  it("puts each step in the column given by its depth, so the flow reads left to right", () => {
    const col = (id: string) => l.nodes.find((x) => x.id === id)?.column;
    expect([col("start"), col("check"), col("decide"), col("ship"), col("backorder"), col("done")]).toEqual([0, 1, 2, 3, 3, 4]);
    const x = (id: string) => l.nodes.find((q) => q.id === id)?.cx ?? NaN;
    expect(x("check") - x("start")).toBe(COLUMN_GAP);
  });

  it("keeps every coordinate finite and inside the canvas", () => {
    for (const p of l.nodes) {
      for (const v of [p.cx, p.cy, p.w, p.h]) expect(Number.isFinite(v)).toBe(true);
      expect(p.cx - p.w / 2).toBeGreaterThanOrEqual(0);
      expect(p.cx + p.w / 2).toBeLessThanOrEqual(l.width);
      expect(p.cy - p.h / 2).toBeGreaterThanOrEqual(0);
      expect(p.cy + p.h / 2).toBeLessThanOrEqual(l.height);
    }
  });

  it("never overlaps two steps in the same column", () => {
    for (const a of l.nodes) {
      for (const b of l.nodes) {
        if (a.id >= b.id || a.column !== b.column) continue;
        expect(Math.abs(a.cy - b.cy)).toBeGreaterThanOrEqual((a.h + b.h) / 2);
      }
    }
  });

  it("draws forward edges across and backward edges underneath", () => {
    const e = (from: string, to: string) => l.edges.find((x) => x.from === from && x.to === to);
    expect(e("check", "decide")?.back).toBe(false);
    expect(e("backorder", "check")?.back).toBe(true);
    const bottom = Math.max(...l.nodes.map((p) => p.cy + p.h / 2));
    expect(l.height).toBeGreaterThan(bottom); // room was made for the curve
    expect(e("decide", "ship")?.label).toBe("yes");
  });

  it("keeps every edge", () => {
    expect(l.edges).toHaveLength(ORDER.edges.length);
  });

  it("is repeatable", () => {
    expect(layoutProcess(ORDER)).toEqual(l);
  });
});

describe("layoutProcess edge cases", () => {
  it("puts unreachable steps in a last column and marks them", () => {
    const m: ProcessModel = { name: "P", nodes: [n("start", "start", 0), n("a", "end", 1), n("orphan", "task", null)], edges: [{ from: "start", to: "a", label: null }] };
    const l = layoutProcess(m);
    const orphan = l.nodes.find((x) => x.id === "orphan");
    expect(orphan?.reachable).toBe(false);
    expect(orphan?.column).toBe(2);
  });

  it("handles an empty process, a process without start, and a self-loop", () => {
    expect(layoutProcess({ name: "", nodes: [], edges: [] }).width).toBeGreaterThan(0);
    const noStart = layoutProcess({ name: "P", nodes: [n("a", "task", null), n("b", "end", null)], edges: [{ from: "a", to: "b", label: null }] });
    expect(noStart.nodes.every((x) => !x.reachable)).toBe(true);
    const loop = layoutProcess({ name: "P", nodes: [n("start", "start", 0), n("a", "task", 1)], edges: [{ from: "start", to: "a", label: null }, { from: "a", to: "a", label: null }] });
    expect(loop.edges[1]?.back).toBe(true);
    expect(loop.edges[1]?.path).toMatch(/^M /);
  });

  it("ignores an edge to a step that is not in the node list instead of throwing", () => {
    const l = layoutProcess({ name: "P", nodes: [n("start", "start", 0)], edges: [{ from: "start", to: "ghost", label: null }] });
    expect(l.edges).toHaveLength(0);
  });

  it("scales to a long chain", () => {
    const nodes = Array.from({ length: 2000 }, (_, i) => n(`t${i}`, "task", i));
    const edges = nodes.slice(1).map((x, i) => ({ from: `t${i}`, to: x.id, label: null }));
    const l = layoutProcess({ name: "Big", nodes, edges });
    expect(l.nodes).toHaveLength(2000);
    expect(l.width).toBeGreaterThan(1999 * COLUMN_GAP);
  });
});
