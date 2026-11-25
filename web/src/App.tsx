import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { DiagnosticList, Summary } from "./components/Diagnostics.tsx";
import { Diagram } from "./components/Diagram.tsx";
import { Editor } from "./components/Editor.tsx";
import { EXAMPLES } from "./examples.ts";
import type { Diagnostic } from "./types.ts";
import { loadProcflow, type Procflow } from "./wasm.ts";

type Engine = { state: "loading" } | { state: "ready"; pf: Procflow } | { state: "failed"; message: string };

export default function App() {
  const [engine, setEngine] = useState<Engine>({ state: "loading" });
  const [text, setText] = useState(EXAMPLES[0]?.source ?? "");
  const [note, setNote] = useState("");
  const editor = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    let cancelled = false;
    loadProcflow(`${import.meta.env.BASE_URL}procflow.wasm`).then(
      (pf) => !cancelled && setEngine({ state: "ready", pf }),
      (err: unknown) => !cancelled && setEngine({ state: "failed", message: err instanceof Error ? err.message : String(err) }),
    );
    return () => {
      cancelled = true;
    };
  }, []);

  const analysis = useMemo(() => (engine.state === "ready" ? engine.pf.analyze(text) : null), [engine, text]);

  const select = useCallback((d: Diagnostic) => {
    const el = editor.current;
    if (!el) return;
    el.focus();
    el.setSelectionRange(d.start, d.end); // UTF-16 offsets: exactly what a textarea uses
  }, []);

  const format = () => {
    if (engine.state !== "ready") return;
    const r = engine.pf.format(text);
    if (r.ok) {
      setText(r.text);
      setNote("Formatted.");
    } else {
      setNote(r.reason === "syntax" ? "Fix the syntax errors first: only text that parses can be formatted." : "The text could not be encoded.");
    }
  };

  return (
    <>
      <header>
        <h1>Procflow editor</h1>
        <p>
          A small process language with a Rust parser compiled to WebAssembly. Type on the left; the checks and the diagram update as
          you go.
        </p>
      </header>
      <main>
        <section className="panel" aria-labelledby="source-h">
          <div className="bar">
            <h2 id="source-h">Source</h2>
            <label className="examples">
              Example
              <select
                onChange={(e) => {
                  const ex = EXAMPLES.find((x) => x.id === e.target.value);
                  if (ex) {
                    setText(ex.source);
                    setNote("");
                  }
                }}
                defaultValue={EXAMPLES[0]?.id}
              >
                {EXAMPLES.map((x) => (
                  <option key={x.id} value={x.id}>
                    {x.title}
                  </option>
                ))}
              </select>
            </label>
            <button type="button" onClick={format} disabled={engine.state !== "ready"}>
              Format
            </button>
          </div>
          {engine.state === "loading" && <p role="status">Loading the Rust engine…</p>}
          {engine.state === "failed" && (
            <p role="alert" className="bad">
              Could not load the WebAssembly engine: {engine.message}
            </p>
          )}
          <Editor ref={editor} value={text} onChange={(v) => { setText(v); setNote(""); }} tokens={analysis?.tokens ?? []} diagnostics={analysis?.diagnostics ?? []} />
          <p className="note" role="status" aria-live="polite">
            {note}
          </p>
          {analysis && <Summary diagnostics={analysis.diagnostics} />}
          {analysis && <DiagnosticList diagnostics={analysis.diagnostics} onSelect={select} />}
        </section>
        <section className="panel" aria-labelledby="diagram-h">
          <h2 id="diagram-h">Diagram</h2>
          {analysis && analysis.processes.length === 0 && <p>Nothing to draw yet: add a <code>process</code>.</p>}
          {analysis?.processes.map((p, i) => <Diagram key={`${p.name}-${i}`} model={p} />)}
        </section>
      </main>
      <footer>
        Rust lexer, parser, validator and formatter compiled to a WebAssembly module under 100 kB; React for the interface. No server: everything runs in your browser.
      </footer>
    </>
  );
}
