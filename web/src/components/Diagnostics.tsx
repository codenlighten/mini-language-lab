import type { Diagnostic } from "../types.ts";

interface Props {
  diagnostics: Diagnostic[];
  onSelect: (d: Diagnostic) => void;
}

export function Summary({ diagnostics }: { diagnostics: Diagnostic[] }) {
  const errors = diagnostics.filter((d) => d.severity === "error").length;
  const warnings = diagnostics.length - errors;
  const text =
    diagnostics.length === 0
      ? "No problems found."
      : `${errors} error${errors === 1 ? "" : "s"}, ${warnings} warning${warnings === 1 ? "" : "s"}.`;
  return (
    <p id="diagnostics-summary" role="status" className={diagnostics.length === 0 ? "ok" : errors > 0 ? "bad" : "warn"}>
      {text}
    </p>
  );
}

export function DiagnosticList({ diagnostics, onSelect }: Props) {
  if (diagnostics.length === 0) return null;
  return (
    <ul className="diagnostics" aria-label="Problems">
      {diagnostics.map((d, i) => (
        <li key={`${d.code}-${d.start}-${i}`}>
          <button type="button" className={`diag ${d.severity}`} onClick={() => onSelect(d)}>
            <span className="where">
              {d.line + 1}:{d.column + 1}
            </span>
            <span className="code">{d.code}</span>
            <span className="msg">
              {d.message}
              {d.hint && <em> {d.hint}</em>}
            </span>
          </button>
        </li>
      ))}
    </ul>
  );
}
