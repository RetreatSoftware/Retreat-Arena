#!/usr/bin/env node
// Builds the game for the browser into target/web/: the .wasm and its JavaScript, the engine's page
// (renamed to this game) and scripts/verify/ (so `?commands=` can fetch them).
//
//   node scripts/web/build.mjs
//
// Needs: `rustup target add wasm32-unknown-unknown`, and wasm-bindgen-cli at the version of the
// wasm-bindgen crate in Cargo.lock (cargo install wasm-bindgen-cli --version <x> --locked).
import { execFileSync } from "node:child_process";
import { cpSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { ARENA_ROOT, engineCheckout } from "./engine_checkout.mjs";

const run = (command, args) => execFileSync(command, args, { cwd: ARENA_ROOT, stdio: "inherit" });
const web = join(ARENA_ROOT, "target", "web");

run("cargo", ["build", "--target", "wasm32-unknown-unknown", "--profile", "web-dev"]);
rmSync(web, { recursive: true, force: true });
mkdirSync(join(web, "scripts"), { recursive: true });
run("wasm-bindgen", [
  "--target", "web", "--no-typescript", "--remove-name-section", "--out-dir", web,
  join("target", "wasm32-unknown-unknown", "web-dev", "retreat_arena.wasm"),
]);

// The page names the engine's sandbox in two places; a change there must fail here, not in the browser.
let page = readFileSync(join(engineCheckout(), "web", "index.html"), "utf8");
for (const [from, to] of [["./retreat_sandbox.js", "./retreat_arena.js"], ["<title>Retreat Engine Sandbox</title>", "<title>Retreat Arena</title>"]]) {
  if (!page.includes(from)) throw new Error(`the engine's web/index.html no longer contains \`${from}\`; update scripts/web/build.mjs`);
  page = page.replace(from, to);
}
writeFileSync(join(web, "index.html"), page);
cpSync(join(ARENA_ROOT, "scripts", "verify"), join(web, "scripts", "verify"), { recursive: true });
console.log("Built target/web. Serve: python -m http.server 8090 --directory target/web");
