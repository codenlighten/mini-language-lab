import { useId } from "react";
import { wrapLabel } from "../label.ts";
import { layoutProcess, type PositionedNode } from "../layout.ts";
import type { ProcessModel } from "../types.ts";

const MAX_LABEL = 17;
/** The diagram shrinks to fit its panel, but not below this scale; past that it scrolls. */
const MIN_SCALE = 0.75;
const LINE_HEIGHT = 14;
const short = (s: string) => wrapLabel(s, MAX_LABEL, 1)[0] ?? "";

function Shape({ n }: { n: PositionedNode }) {
  const cls = `node ${n.kind}${n.reachable ? "" : " unreachable"}`;
  const text = n.kind === "start" ? "start" : (n.label ?? n.id);
  const title = n.kind === "start" ? "start" : `${n.id}${n.label ? `: ${n.label}` : ""}`;
  const x = n.cx - n.w / 2;
  const y = n.cy - n.h / 2;
  // A task's box has room for two lines; a diamond narrows too fast for more than one.
  const lines = wrapLabel(text, MAX_LABEL, n.kind === "task" ? 2 : 1);
  const firstLine = n.cy + 4 - ((lines.length - 1) * LINE_HEIGHT) / 2;
  return (
    <g className={cls} data-node={n.id}>
      <title>{title}</title>
      {n.kind === "task" && <rect x={x} y={y} width={n.w} height={n.h} rx={8} />}
      {n.kind === "gateway" && <polygon points={`${n.cx},${y} ${x + n.w},${n.cy} ${n.cx},${y + n.h} ${x},${n.cy}`} />}
      {(n.kind === "start" || n.kind === "end") && <circle cx={n.cx} cy={n.cy} r={n.w / 2} />}
      {n.kind === "end" && <circle cx={n.cx} cy={n.cy} r={n.w / 2 - 6} className="inner" />}
      {n.kind === "start" && <text x={n.cx} y={n.cy + 4} textAnchor="middle" className="small">start</text>}
      {n.kind !== "start" && n.kind !== "end" && (
        <text x={n.cx} y={firstLine} textAnchor="middle">
          {lines.map((line, i) => (
            <tspan key={i} x={n.cx} dy={i === 0 ? 0 : LINE_HEIGHT}>
              {line}
            </tspan>
          ))}
        </text>
      )}
      {n.kind === "end" && (
        <text x={n.cx} y={n.cy + n.h / 2 + 16} textAnchor="middle" className="small">
          {short(n.label ?? n.id)}
        </text>
      )}
    </g>
  );
}

export function Diagram({ model }: { model: ProcessModel }) {
  const layout = layoutProcess(model);
  const arrow = `arrow-${useId().replace(/:/g, "_")}`; // unique per diagram, even if two processes share a name
  const summary = `Diagram of process ${model.name || "(unnamed)"}: ${model.nodes.length} steps and ${model.edges.length} connections.`;
  return (
    <figure className="diagram">
      <figcaption>{model.name || "(unnamed process)"}</figcaption>
      <div className="scroll">
        <svg
          viewBox={`0 0 ${layout.width} ${layout.height}`}
          style={{ width: "100%", maxWidth: layout.width, minWidth: layout.width * MIN_SCALE }}
          role="img"
          aria-label={summary}
        >
          <defs>
            <marker id={arrow} viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
              <path d="M 0 0 L 10 5 L 0 10 z" className="arrowhead" />
            </marker>
          </defs>
          {layout.edges.map((e, i) => (
            <g key={i} className={`edge${e.back ? " back" : ""}`} data-edge={`${e.from}->${e.to}`}>
              <path d={e.path} fill="none" markerEnd={`url(#${arrow})`} />
            </g>
          ))}
          {/* Labels after every connector, so that no line is painted over them. */}
          {layout.edges.map(
            (e, i) =>
              e.label && (
                <text key={i} x={e.labelX} y={e.labelY} textAnchor="middle" className="edge-label">
                  {e.label}
                </text>
              ),
          )}
          {layout.nodes.map((n) => (
            <Shape key={n.id} n={n} />
          ))}
        </svg>
      </div>
      <details>
        <summary>Text version of this diagram</summary>
        <ul>
          {model.nodes.map((n) => (
            <li key={n.id}>
              {n.kind} <code>{n.id}</code>
              {n.label ? ` "${n.label}"` : ""}
              {n.depth === null ? " (cannot be reached)" : ""}
              {model.edges
                .filter((e) => e.from === n.id)
                .map((e) => ` → ${e.label ? `${e.label}: ` : ""}${e.to}`)
                .join(";")}
            </li>
          ))}
        </ul>
      </details>
    </figure>
  );
}
