// Copies the release WASM file where Vite serves it from. Fails loudly if the build is missing.
import { copyFileSync, existsSync, mkdirSync, statSync } from "node:fs";
import { dirname } from "node:path";
import { fileURLToPath } from "node:url";

const from = fileURLToPath(new URL("../../target/wasm32-unknown-unknown/release/procflow_wasm.wasm", import.meta.url));
const to = fileURLToPath(new URL("../public/procflow.wasm", import.meta.url));
if (!existsSync(from)) {
  console.error(`Missing ${from}. Run: npm run build:wasm`);
  process.exit(1);
}
// Git does not keep empty directories, so on a fresh clone web/public does not exist yet.
mkdirSync(dirname(to), { recursive: true });
copyFileSync(from, to);
console.log(`copied procflow.wasm (${statSync(to).size} bytes)`);
