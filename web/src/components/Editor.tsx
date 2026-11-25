import { forwardRef, useMemo, useRef } from "react";
import { buildSegments } from "../highlight.ts";
import type { Diagnostic, Token } from "../types.ts";

interface Props {
  value: string;
  onChange: (value: string) => void;
  tokens: Token[];
  diagnostics: Diagnostic[];
}

/**
 * A small code editor: a real <textarea> (so selection, undo, IME and screen readers all behave
 * natively) laid over a <pre> that draws the same text with colours and underlines. Both use the same
 * font and padding, and the <pre> scrolls with the textarea.
 */
export const Editor = forwardRef<HTMLTextAreaElement, Props>(function Editor({ value, onChange, tokens, diagnostics }, ref) {
  const preRef = useRef<HTMLPreElement>(null);
  const gutterRef = useRef<HTMLDivElement>(null);
  const segments = useMemo(() => buildSegments(value, tokens, diagnostics), [value, tokens, diagnostics]);
  const lines = value.split("\n").length;

  return (
    <div className="editor">
      <div className="gutter" ref={gutterRef} aria-hidden="true">
        {Array.from({ length: lines }, (_, i) => (
          <div key={i}>{i + 1}</div>
        ))}
      </div>
      <div className="surface">
        <pre className="highlight" ref={preRef} aria-hidden="true">
          {segments.map((s) => (
            <span
              key={s.start}
              className={[s.kind ? `tok-${s.kind}` : "", s.severity ? `sq-${s.severity}` : ""].filter(Boolean).join(" ") || undefined}
              data-severity={s.severity ?? undefined}
            >
              {s.text}
            </span>
          ))}
          {/* A trailing newline would otherwise collapse and make the last line shorter than the textarea's. */}
          {"\n"}
        </pre>
        <textarea
          ref={ref}
          className="input"
          aria-label="Procflow source"
          aria-describedby="diagnostics-summary"
          spellCheck={false}
          autoCapitalize="off"
          autoCorrect="off"
          wrap="off"
          value={value}
          onChange={(e) => onChange(e.target.value)}
          onScroll={(e) => {
            const t = e.currentTarget;
            if (preRef.current) {
              preRef.current.scrollTop = t.scrollTop;
              preRef.current.scrollLeft = t.scrollLeft;
            }
            if (gutterRef.current) gutterRef.current.scrollTop = t.scrollTop;
          }}
        />
      </div>
    </div>
  );
});
