import type { Diagnostic, Token, TokenClass } from "./types.ts";

export interface Segment {
  text: string;
  start: number;
  end: number;
  kind: TokenClass | null;
  severity: "error" | "warning" | null;
}

const isHigh = (c: number) => c >= 0xd800 && c <= 0xdbff;
const isLow = (c: number) => c >= 0xdc00 && c <= 0xdfff;

/**
 * The range to underline for a diagnostic. A zero-width diagnostic (for example "missing `}`" at the
 * end of the file) is widened to one character so it is visible, without ever splitting a surrogate
 * pair (an emoji is two UTF-16 units and must stay whole).
 */
export function displayRange(d: Pick<Diagnostic, "start" | "end">, text: string): [number, number] {
  const len = text.length;
  let start = Math.min(Math.max(d.start, 0), len);
  let end = Math.min(Math.max(d.end, start), len);
  if (end > start || len === 0) return [start, end];
  if (start < len) {
    end = start + 1;
    if (isHigh(text.charCodeAt(start)) && isLow(text.charCodeAt(start + 1))) end = start + 2;
  } else {
    start = len - 1;
    if (isLow(text.charCodeAt(start)) && start > 0 && isHigh(text.charCodeAt(start - 1))) start -= 1;
    end = len;
  }
  return [start, end];
}

/**
 * Cuts the text into runs that share a token class and an underline severity, so the editor can
 * draw it with a few spans. The segments always join back into exactly `text`.
 */
export function buildSegments(text: string, tokens: readonly Token[], diagnostics: readonly Diagnostic[]): Segment[] {
  const len = text.length;
  const points = new Set<number>([0, len]);
  const clamp = (n: number) => Math.min(Math.max(n, 0), len);
  for (const t of tokens) {
    points.add(clamp(t.start));
    points.add(clamp(t.end));
  }
  const ranges = diagnostics.map((d) => ({ severity: d.severity, range: displayRange(d, text) }));
  for (const r of ranges) {
    points.add(r.range[0]);
    points.add(r.range[1]);
  }
  const sorted = [...points].sort((a, b) => a - b);
  const index = new Map(sorted.map((p, i) => [p, i] as const));

  // Difference arrays: how many errors / warnings cover each interval between two points.
  const errors = new Array<number>(sorted.length + 1).fill(0);
  const warnings = new Array<number>(sorted.length + 1).fill(0);
  for (const { severity, range } of ranges) {
    const [a, b] = [index.get(range[0]), index.get(range[1])];
    if (a === undefined || b === undefined || a >= b) continue;
    const arr = severity === "error" ? errors : warnings;
    arr[a] = (arr[a] ?? 0) + 1;
    arr[b] = (arr[b] ?? 0) - 1;
  }

  const out: Segment[] = [];
  let e = 0;
  let w = 0;
  let ti = 0;
  const toks = [...tokens].sort((a, b) => a.start - b.start);
  for (let i = 0; i + 1 < sorted.length; i++) {
    e += errors[i] ?? 0;
    w += warnings[i] ?? 0;
    const a = sorted[i] as number;
    const b = sorted[i + 1] as number;
    while (ti < toks.length && (toks[ti] as Token).end <= a) ti++;
    const t = toks[ti];
    const kind = t && t.start <= a && b <= t.end ? t.kind : null;
    const severity = e > 0 ? "error" : w > 0 ? "warning" : null;
    const last = out[out.length - 1];
    if (last && last.kind === kind && last.severity === severity) {
      last.end = b;
      last.text = text.slice(last.start, b);
    } else {
      out.push({ text: text.slice(a, b), start: a, end: b, kind, severity });
    }
  }
  return out;
}
