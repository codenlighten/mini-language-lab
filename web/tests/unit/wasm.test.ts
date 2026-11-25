import { existsSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { beforeAll, describe, expect, it } from "vitest";
import { instantiate, wrap, type Procflow } from "../../src/wasm.ts";

const WASM = fileURLToPath(new URL("../../../target/wasm32-unknown-unknown/release/procflow_wasm.wasm", import.meta.url));

const SAMPLE = `process Order {
  start -> check
  task check "Check stock" -> decide
  gateway decide "In stock?" {
    yes -> ship,
    no -> backorder
  }
  task ship "Ship" -> done
  task backorder "Order" -> check
  end done
}
`;

let pf: Procflow;
beforeAll(async () => {
  if (!existsSync(WASM)) {
    throw new Error(`The WebAssembly build is missing at ${WASM}. Run: npm run build:wasm`);
  }
  pf = await instantiate(readFileSync(WASM));
});

describe("the real WebAssembly module, called from JavaScript", () => {
  it("analyzes a valid program with no problems and builds the graph", () => {
    const a = pf.analyze(SAMPLE);
    expect(a.diagnostics).toEqual([]);
    expect(a.processes).toHaveLength(1);
    expect(a.processes[0]?.nodes.map((x) => x.id)).toEqual(["start", "check", "decide", "ship", "backorder", "done"]);
    expect(a.processes[0]?.edges.find((e) => e.from === "decide" && e.to === "ship")?.label).toBe("yes");
  });

  it("reports a typo with a suggestion and exact UTF-16 offsets that slice the text", () => {
    const text = "process P {\n  start -> a\n  task a -> sihp\n  task ship -> done\n  end done\n}";
    const d = pf.analyze(text).diagnostics.find((x) => x.code === "E102");
    expect(d?.hint).toBe("did you mean `ship`?");
    expect(text.slice(d?.start, d?.end)).toBe("sihp");
    expect([d?.line, d?.column]).toEqual([2, 12]);
  });

  it("keeps offsets correct after emoji and accents (UTF-16, not bytes)", () => {
    const text = "// 📦 café\nprocess P {\n  start -> nope\n  end e\n}";
    const a = pf.analyze(text);
    const d = a.diagnostics.find((x) => x.code === "E102");
    expect(text.slice(d?.start, d?.end)).toBe("nope");
    for (const t of a.tokens) expect(t.end).toBeLessThanOrEqual(text.length);
    const comment = a.tokens.find((t) => t.kind === "comment");
    expect(text.slice(comment?.start, comment?.end)).toBe("// 📦 café");
  });

  it("formats, or says why it cannot", () => {
    const ok = pf.format("process P{start->a end a}");
    expect(ok).toEqual({ ok: true, text: "process P {\n  start -> a\n  end a\n}\n" });
    expect(pf.format("process P { start a }")).toEqual({ ok: false, reason: "syntax" });
  });

  it("handles empty text, a lone surrogate and very odd input without throwing", () => {
    expect(pf.analyze("").diagnostics).toEqual([]);
    expect(() => pf.analyze("\ud800 process")).not.toThrow(); // TextEncoder replaces the lone surrogate
    expect(() => pf.analyze("\u0000   \"\\")).not.toThrow();
  });

  it("refuses input over 1 MiB with a clear diagnostic", () => {
    const a = pf.analyze("x".repeat(1_048_577));
    expect(a.diagnostics.map((d) => d.code)).toEqual(["E900"]);
  });

  it("survives memory growth: results stay correct over many large calls", () => {
    // Large inputs make the module grow its memory, which detaches earlier ArrayBuffer views.
    const big = "// pad\n".repeat(60_000); // about 420 KB
    const src = `${big}process P { start -> a\n end a }`;
    const expected = pf.analyze(src).diagnostics;
    for (let i = 0; i < 40; i++) expect(pf.analyze(src).diagnostics).toEqual(expected);
    expect(pf.analyze(SAMPLE).diagnostics).toEqual([]);
  });

  it("gives the same answer every time", () => {
    expect(pf.analyze(SAMPLE)).toEqual(pf.analyze(SAMPLE));
  });
});

describe("wrap", () => {
  it("rejects a module that does not export what the wrapper needs, naming the missing export", async () => {
    // (module (memory (export "memory") 1))
    const bytes = new Uint8Array([0, 97, 115, 109, 1, 0, 0, 0, 5, 3, 1, 0, 1, 7, 10, 1, 6, 109, 101, 109, 111, 114, 121, 2, 0]);
    const { instance } = await WebAssembly.instantiate(bytes, {});
    expect(() => wrap(instance)).toThrow(/does not export "alloc"/);
  });
});
