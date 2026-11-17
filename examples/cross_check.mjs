// Cross-check this crate against the JavaScript pump-curve package on the same account bytes.
//
//   git clone https://github.com/blixvip/pump-curve ../pump-curve
//   node examples/cross_check.mjs                 # or: PUMP_CURVE_JS=/path/to/pump-curve node examples/cross_check.mjs
//
// It runs `cargo run -q --example cross_check`, decodes every curve's base64 with pump-curve's decodeCurve(), and
// compares price, market cap, progress and migration market cap. Exit code 1 on any mismatch.
// Node 20+, no dependencies.
import { execFileSync } from "node:child_process";
import { existsSync } from "node:fs";
import { resolve, join } from "node:path";
import { pathToFileURL, fileURLToPath } from "node:url";

const root = resolve(fileURLToPath(new URL("..", import.meta.url)));
const jsRoot = resolve(process.env.PUMP_CURVE_JS || join(root, "..", "pump-curve"));
const entry = existsSync(join(jsRoot, "src", "index.js")) ? join(jsRoot, "src", "index.js") : jsRoot;
if (!existsSync(entry)) {
  console.error(`pump-curve (JS) not found at ${jsRoot}\nclone it: git clone https://github.com/blixvip/pump-curve ../pump-curve`);
  process.exit(2);
}
const { decodeCurve } = await import(pathToFileURL(entry).href);

const rust = JSON.parse(execFileSync("cargo", ["run", "-q", "--example", "cross_check"], { cwd: root, encoding: "utf8" }));

// Floats are computed in a different order in each language, so allow a relative 1e-9. Progress: the crate floors
// to whole basis points, pump-curve rounds to two decimals, so they can differ by up to 0.01.
const close = (a, b, tol, rel) => (a == null || b == null ? a == b : Math.abs(a - b) <= (rel ? tol * Math.max(Math.abs(a), Math.abs(b)) : tol));
const fields = [["priceSol", 1e-9, true], ["marketCapSol", 1e-9, true], ["migrationMarketCapSol", 1e-9, true], ["progressPct", 0.0100001, false]];

let bad = 0;
for (const r of rust) {
  const js = decodeCurve(r.base64);
  const diffs = fields.filter(([f, tol, rel]) => !close(r[f], js[f], tol, rel)).map(([f]) => `${f}: rust ${r[f]} js ${js[f]}`);
  if (r.complete !== js.complete) diffs.push(`complete: rust ${r.complete} js ${js.complete}`);
  bad += diffs.length ? 1 : 0;
  const shown = r.complete ? `complete, progress ${r.progressPct}% (js ${js.progressPct}%)` : r.marketCapSol == null ? "not priced in SOL (both null)" : `cap ${r.marketCapSol.toFixed(4)} SOL (js ${js.marketCapSol.toFixed(4)}), progress ${r.progressPct}% (js ${js.progressPct}%)`;
  console.log(`${diffs.length ? "DIFF" : "ok  "}  ${r.name}: ${shown}${diffs.length ? `\n      ${diffs.join("\n      ")}` : ""}`);
}
console.log(bad ? `\n${bad} of ${rust.length} curves differ` : `\nall ${rust.length} curves agree`);
process.exit(bad ? 1 : 0);
