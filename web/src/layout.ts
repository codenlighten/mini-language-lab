import type { GraphNode, NodeKind, ProcessModel } from "./types.ts";

export interface PositionedNode {
  id: string;
  kind: NodeKind;
  label: string | null;
  /** Centre of the node. */
  cx: number;
  cy: number;
  w: number;
  h: number;
  row: number;
  reachable: boolean;
}

export interface PositionedEdge {
  from: string;
  to: string;
  label: string | null;
  /** SVG path data. */
  path: string;
  labelX: number;
  labelY: number;
  /** True when the edge goes back (or sideways), and so is routed round the right-hand side. */
  back: boolean;
}

export interface Layout {
  nodes: PositionedNode[];
  edges: PositionedEdge[];
  width: number;
  height: number;
}

export const COLUMN_GAP = 156;
export const ROW_GAP = 96;
export const MARGIN = 24;
/** How far the first backward edge swings out to the right, and how much further each next one does. */
const SWING = 48;
const SWING_STEP = 18;
const SIZES: Record<NodeKind, [number, number]> = {
  start: [44, 44],
  end: [44, 44],
  task: [132, 48],
  gateway: [132, 68],
};
const MAX_W = 132;
const MAX_H = 68;

/**
 * Rows for the steps that cannot be reached from `start`, counted from 0. They keep their own shape: a step
 * with no unreachable predecessor is on row 0 and each later step is one row below the first step that
 * leads to it. In a loop with no such entry, the first step in source order is the entry.
 */
function detachedRows(model: ProcessModel): Map<string, number> {
  const detached = model.nodes.filter((n) => n.depth === null).map((n) => n.id);
  const inside = new Set(detached);
  const next = new Map<string, string[]>(detached.map((id) => [id, []]));
  const entered = new Set<string>();
  for (const e of model.edges) {
    if (e.from === e.to || !inside.has(e.from) || !inside.has(e.to)) continue;
    next.get(e.from)?.push(e.to);
    entered.add(e.to);
  }
  const rows = new Map<string, number>();
  const spread = (roots: string[]) => {
    const queue = roots.filter((id) => !rows.has(id));
    for (const id of queue) rows.set(id, 0);
    for (let i = 0; i < queue.length; i++) {
      const id = queue[i] as string;
      for (const to of next.get(id) ?? []) {
        if (rows.has(to)) continue;
        rows.set(to, (rows.get(id) as number) + 1);
        queue.push(to);
      }
    }
  };
  spread(detached.filter((id) => !entered.has(id)));
  for (const id of detached) spread([id]); // whatever is left sits in loops
  return rows;
}

/**
 * Lays a process out in rows: a step's row is its distance from `start`, so the flow reads top to bottom and
 * fits a narrow panel. Steps that cannot be reached follow below, keeping their own structure. Edges that do
 * not move down are drawn as curves round the right-hand side. The result depends only on the input, so it is
 * repeatable.
 */
export function layoutProcess(model: ProcessModel): Layout {
  const maxDepth = model.nodes.reduce((m, n) => Math.max(m, n.depth ?? -1), -1);
  const detached = detachedRows(model);
  const rowOf = (n: GraphNode) => (n.depth === null ? maxDepth + 1 + (detached.get(n.id) ?? 0) : n.depth);

  const rows = Math.max(...model.nodes.map((n) => rowOf(n) + 1), 1);
  const byRow: GraphNode[][] = Array.from({ length: rows }, () => []);
  for (const n of model.nodes) (byRow[rowOf(n)] as GraphNode[]).push(n);
  const widest = Math.max(...byRow.map((r) => r.length), 1);

  // Backward and sideways edges swing out to the right, so their ends go on the right of their row; otherwise
  // the curve would run through the neighbours. Source order is kept among the rest.
  const rowById = new Map(model.nodes.map((n) => [n.id, rowOf(n)]));
  const outer = new Set<string>();
  for (const e of model.edges) {
    const from = rowById.get(e.from);
    const to = rowById.get(e.to);
    if (from === undefined || to === undefined || to > from) continue;
    outer.add(e.from);
    outer.add(e.to);
  }
  for (const row of byRow) row.sort((a, b) => Number(outer.has(a.id)) - Number(outer.has(b.id)));

  const nodes: PositionedNode[] = [];
  const at = new Map<string, PositionedNode>();
  byRow.forEach((row, ri) => {
    const left = MARGIN + MAX_W / 2 + (widest - row.length) * (COLUMN_GAP / 2);
    row.forEach((n, ci) => {
      const [w, h] = SIZES[n.kind];
      const p: PositionedNode = {
        id: n.id,
        kind: n.kind,
        label: n.label,
        cx: left + ci * COLUMN_GAP,
        cy: MARGIN + MAX_H / 2 + ri * ROW_GAP,
        w,
        h,
        row: ri,
        reachable: n.depth !== null,
      };
      nodes.push(p);
      at.set(p.id, p);
    });
  });

  // An end step has its label under the circle, which needs room too.
  const bottom = Math.max(...nodes.map((n) => n.cy + n.h / 2 + (n.kind === "end" ? 20 : 0)), MARGIN);
  const right = MARGIN * 2 + MAX_W + (widest - 1) * COLUMN_GAP - MARGIN;
  let backCount = 0;
  const edges: PositionedEdge[] = [];
  for (const e of model.edges) {
    const a = at.get(e.from);
    const b = at.get(e.to);
    if (!a || !b) continue; // cannot happen for output of the Rust side; skip rather than throw
    if (b.row > a.row) {
      const sy = a.cy + a.h / 2;
      const ey = b.cy - b.h / 2;
      const my = (sy + ey) / 2;
      edges.push({
        from: e.from,
        to: e.to,
        label: e.label,
        path: `M ${a.cx} ${sy} C ${a.cx} ${my}, ${b.cx} ${my}, ${b.cx} ${ey}`,
        labelX: (a.cx + b.cx) / 2,
        labelY: my + 4,
        back: false,
      });
    } else {
      const swing = right + SWING + SWING_STEP * backCount++;
      // Leave a little below the middle and arrive a little above it, so a step looping to itself shows.
      const sx = a.cx + a.w / 2;
      const sy = a.cy + 8;
      const ex = b.cx + b.w / 2;
      const ey = b.cy - 8;
      edges.push({
        from: e.from,
        to: e.to,
        label: e.label,
        path: `M ${sx} ${sy} C ${swing} ${sy}, ${swing} ${ey}, ${ex} ${ey}`,
        // The curve's furthest point is three quarters of the way to its control points.
        labelX: 0.75 * swing + 0.25 * ((sx + ex) / 2),
        labelY: (sy + ey) / 2 - 4,
        back: true,
      });
    }
  }
  return {
    nodes,
    edges,
    width: right + MARGIN + (backCount > 0 ? SWING + SWING_STEP * backCount : 0),
    height: bottom + MARGIN,
  };
}
