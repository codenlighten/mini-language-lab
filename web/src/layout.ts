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
  column: number;
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
  /** True when the edge goes back (or sideways), and so is drawn underneath the nodes. */
  back: boolean;
}

export interface Layout {
  nodes: PositionedNode[];
  edges: PositionedEdge[];
  width: number;
  height: number;
}

export const COLUMN_GAP = 190;
export const ROW_GAP = 96;
export const MARGIN = 40;
const SIZES: Record<NodeKind, [number, number]> = {
  start: [44, 44],
  end: [44, 44],
  task: [132, 48],
  gateway: [132, 68],
};
const MAX_W = 132;

/**
 * Lays a process out in columns: a step's column is its distance from `start`, so the flow reads
 * left to right. Steps that cannot be reached go in a last column. Edges that do not move right are
 * drawn as curves underneath the steps. The result depends only on the input, so it is repeatable.
 */
export function layoutProcess(model: ProcessModel): Layout {
  const maxDepth = model.nodes.reduce((m, n) => Math.max(m, n.depth ?? -1), -1);
  const hasUnreachable = model.nodes.some((n) => n.depth === null);
  const columns = Math.max(maxDepth + 1 + (hasUnreachable ? 1 : 0), 1);
  const columnOf = (n: GraphNode) => (n.depth === null ? maxDepth + 1 : n.depth);

  const byColumn: GraphNode[][] = Array.from({ length: columns }, () => []);
  for (const n of model.nodes) (byColumn[columnOf(n)] as GraphNode[]).push(n);
  const tallest = Math.max(...byColumn.map((c) => c.length), 1);

  const nodes: PositionedNode[] = [];
  const at = new Map<string, PositionedNode>();
  byColumn.forEach((col, ci) => {
    const top = MARGIN + (tallest - col.length) * (ROW_GAP / 2) + ROW_GAP / 2;
    col.forEach((n, ri) => {
      const [w, h] = SIZES[n.kind];
      const p: PositionedNode = {
        id: n.id,
        kind: n.kind,
        label: n.label,
        cx: MARGIN + MAX_W / 2 + ci * COLUMN_GAP,
        cy: top + ri * ROW_GAP,
        w,
        h,
        column: ci,
        reachable: n.depth !== null,
      };
      nodes.push(p);
      at.set(p.id, p);
    });
  });

  const bottom = Math.max(...nodes.map((n) => n.cy + n.h / 2), MARGIN);
  let backCount = 0;
  const edges: PositionedEdge[] = [];
  for (const e of model.edges) {
    const a = at.get(e.from);
    const b = at.get(e.to);
    if (!a || !b) continue; // cannot happen for output of the Rust side; skip rather than throw
    if (b.column > a.column) {
      const sx = a.cx + a.w / 2;
      const ex = b.cx - b.w / 2;
      const mx = (sx + ex) / 2;
      edges.push({
        from: e.from,
        to: e.to,
        label: e.label,
        path: `M ${sx} ${a.cy} C ${mx} ${a.cy}, ${mx} ${b.cy}, ${ex} ${b.cy}`,
        labelX: mx,
        labelY: (a.cy + b.cy) / 2 - 8,
        back: false,
      });
    } else {
      const drop = bottom + 34 + 14 * backCount++;
      const sx = a.cx;
      const sy = a.cy + a.h / 2;
      const ex = b.cx;
      const ey = b.cy + b.h / 2;
      edges.push({
        from: e.from,
        to: e.to,
        label: e.label,
        path: `M ${sx} ${sy} C ${sx - 30} ${drop}, ${ex + 30} ${drop}, ${ex} ${ey}`,
        labelX: (sx + ex) / 2,
        labelY: drop - 4,
        back: true,
      });
    }
  }
  return {
    nodes,
    edges,
    width: MARGIN * 2 + MAX_W + (columns - 1) * COLUMN_GAP,
    height: bottom + MARGIN + (backCount > 0 ? 34 + 14 * backCount : 0),
  };
}
