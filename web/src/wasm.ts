import type { Analysis } from "./types.ts";

interface Exports {
  memory: WebAssembly.Memory;
  alloc(len: number): number;
  dealloc(ptr: number, len: number): void;
  output_ptr(): number;
  analyze(ptr: number, len: number): number;
  format_source(ptr: number, len: number): number;
}

export type FormatResult = { ok: true; text: string } | { ok: false; reason: "syntax" | "encoding" };

export interface Procflow {
  analyze(text: string): Analysis;
  format(text: string): FormatResult;
}

const encoder = new TextEncoder();
const decoder = new TextDecoder("utf-8", { fatal: true });

const REQUIRED = ["memory", "alloc", "dealloc", "output_ptr", "analyze", "format_source"] as const;

/** Wraps an instantiated module. Exported for tests; the app uses `loadProcflow`. */
export function wrap(instance: WebAssembly.Instance): Procflow {
  const ex = instance.exports as unknown as Exports;
  for (const name of REQUIRED) {
    if (!(name in ex)) throw new Error(`The WebAssembly module does not export "${name}".`);
  }

  /** Runs one export over `text` and returns its status and output bytes (if any). */
  function call(fn: (ptr: number, len: number) => number, text: string): { status: number; out: string } {
    const bytes = encoder.encode(text);
    const ptr = ex.alloc(bytes.length);
    try {
      new Uint8Array(ex.memory.buffer, ptr, bytes.length).set(bytes);
      const status = fn(ptr, bytes.length);
      // Take the view only now: the call may have grown memory, which detaches older views.
      const out = status >= 0 ? decoder.decode(new Uint8Array(ex.memory.buffer, ex.output_ptr(), status)) : "";
      return { status, out };
    } finally {
      ex.dealloc(ptr, bytes.length);
    }
  }

  return {
    analyze(text) {
      const { status, out } = call(ex.analyze, text);
      if (status < 0) throw new Error("The text could not be encoded as UTF-8.");
      return JSON.parse(out) as Analysis;
    },
    format(text) {
      const { status, out } = call(ex.format_source, text);
      if (status === -1) return { ok: false, reason: "encoding" };
      if (status < 0) return { ok: false, reason: "syntax" };
      return { ok: true, text: out };
    },
  };
}

export async function instantiate(bytes: BufferSource): Promise<Procflow> {
  const { instance } = await WebAssembly.instantiate(bytes, {});
  return wrap(instance);
}

/** Loads the module in the browser, streaming it when the server sends the right MIME type. */
export async function loadProcflow(url: string): Promise<Procflow> {
  const res = await fetch(url);
  if (!res.ok) throw new Error(`Could not load ${url}: HTTP ${res.status}`);
  return instantiate(await res.arrayBuffer());
}
