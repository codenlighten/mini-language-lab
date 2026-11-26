import { describe, expect, it } from "vitest";
import { COLUMN_GAP, ROW_GAP, layoutProcess } from "../../src/layout.ts";
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

  it("puts each step in the row given by its depth, so the flow reads top to bottom", () => {
    const row = (id: string) => l.nodes.find((x) => x.id === id)?.row;
    expect([row("start"), row("check"), row("decide"), row("ship"), row("backorder"), row("done")]).toEqual([0, 1, 2, 3, 3, 4]);
    const at = (id: string) => l.nodes.find((q) => q.id === id)!;
    expect(at("check").cy - at("start").cy).toBe(ROW_GAP);
    expect(at("backorder").cx - at("ship").cx).toBe(COLUMN_GAP);
    expect(at("decide").cx).toBe((at("ship").cx + at("backorder").cx) / 2); // a short row is centred
  });

  it("is narrow enough to show the example in the editor's panel without scrolling", () => {
    expect(l.width).toBeLessThanOrEqual(520);
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

  it("never overlaps two steps in the same row", () => {
    for (const a of l.nodes) {
      for (const b of l.nodes) {
        if (a.id >= b.id || a.row !== b.row) continue;
        expect(Math.abs(a.cx - b.cx)).toBeGreaterThanOrEqual((a.w + b.w) / 2);
      }
    }
  });

  it("draws forward edges downwards and backward edges round the right", () => {
    const e = (from: string, to: string) => l.edges.find((x) => x.from === from && x.to === to);
    expect(e("check", "decide")?.back).toBe(false);
    expect(e("backorder", "check")?.back).toBe(true);
    const right = Math.max(...l.nodes.map((p) => p.cx + p.w / 2));
    const back = e("backorder", "check")!;
    expect(back.labelX).toBeGreaterThan(right); // clear of every step
    expect(back.labelX).toBeLessThan(l.width); // and room was made for it
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
  it("puts unreachable steps below the reachable ones and marks them", () => {
    const m: ProcessModel = { name: "P", nodes: [n("start", "start", 0), n("a", "end", 1), n("orphan", "task", null)], edges: [{ from: "start", to: "a", label: null }] };
    const l = layoutProcess(m);
    const orphan = l.nodes.find((x) => x.id === "orphan");
    expect(orphan?.reachable).toBe(false);
    expect(orphan?.row).toBe(2);
  });

  it("puts the ends of backward edges on the right of their row, clear of neighbours", () => {
    const l = layoutProcess({
      name: "P",
      nodes: [n("start", "start", 0), n("loop1", "task", 1), n("loop2", "task", 2), n("done", "end", 2)],
      edges: [
        { from: "start", to: "loop1", label: null },
        { from: "loop1", to: "loop2", label: null },
        { from: "loop1", to: "done", label: null },
        { from: "loop2", to: "loop1", label: null },
      ],
    });
    const x = (id: string) => l.nodes.find((q) => q.id === id)!.cx;
    expect(x("loop2")).toBeGreaterThan(x("done")); // although it comes first in the source
  });

  it("keeps the shape of a disconnected part instead of piling it into one line", () => {
    // What a single mistyped target does to the order example: everything after `check` is cut off.
    const m: ProcessModel = {
      ...ORDER,
      nodes: ORDER.nodes.map((x) => (x.id === "start" || x.id === "check" ? x : { ...x, depth: null })),
      edges: ORDER.edges.filter((e) => !(e.from === "check" && e.to === "decide")),
    };
    const l = layoutProcess(m);
    const row = (id: string) => l.nodes.find((x) => x.id === id)?.row;
    expect([row("decide"), row("ship"), row("backorder"), row("done")]).toEqual([2, 3, 3, 4]);
    expect(l.edges.filter((e) => e.back).map((e) => e.from)).toEqual(["backorder"]);
  });

  it("gives a disconnected loop an entry and still lays it out in order", () => {
    const m: ProcessModel = {
      name: "P",
      nodes: [n("start", "start", 0), n("a", "task", null), n("b", "task", null), n("c", "task", null)],
      edges: [{ from: "a", to: "b", label: null }, { from: "b", to: "c", label: null }, { from: "c", to: "a", label: null }],
    };
    const l = layoutProcess(m);
    expect(l.nodes.filter((x) => !x.reachable).map((x) => x.row)).toEqual([1, 2, 3]);
  });

  it("makes a step that loops to itself visible", () => {
    const l = layoutProcess({ name: "P", nodes: [n("start", "start", 0), n("a", "task", 1)], edges: [{ from: "a", to: "a", label: null }] });
    const [, sy, ey] = /^M \S+ (\S+) C \S+ \S+ \S+ (\S+), /.exec(l.edges[0]!.path)!.map(Number);
    expect(sy).not.toBe(ey);
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
    expect(l.height).toBeGreaterThan(1999 * ROW_GAP);
  });
});
