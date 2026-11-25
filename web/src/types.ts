export type TokenClass = "keyword" | "name" | "string" | "punct" | "comment" | "error";

/** Offsets are UTF-16 code units, so they index a JavaScript string directly. */
export interface Token {
  kind: TokenClass;
  start: number;
  end: number;
}

export interface Diagnostic {
  code: string;
  severity: "error" | "warning";
  message: string;
  hint: string | null;
  start: number;
  end: number;
  /** Zero-based. */
  line: number;
  column: number;
  endLine: number;
  endColumn: number;
}

export type NodeKind = "start" | "task" | "gateway" | "end";

export interface GraphNode {
  id: string;
  kind: NodeKind;
  label: string | null;
  /** Fewest steps from `start`, or null when unreachable. */
  depth: number | null;
}

export interface GraphEdge {
  from: string;
  to: string;
  label: string | null;
}

export interface ProcessModel {
  name: string;
  nodes: GraphNode[];
  edges: GraphEdge[];
}

export interface Analysis {
  tokens: Token[];
  diagnostics: Diagnostic[];
  processes: ProcessModel[];
}
