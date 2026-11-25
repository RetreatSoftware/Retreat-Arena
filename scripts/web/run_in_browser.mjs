#!/usr/bin/env node
// Runs a command script in the browser build, in headless Chrome or Edge, with the engine's runner
// (its documentation lists the options). Snapshots land in target/snapshots/web/.
//
//   node scripts/web/build.mjs
//   node scripts/web/run_in_browser.mjs scripts/verify/walk.cmds
import { spawnSync } from "node:child_process";
import { join } from "node:path";
import { ARENA_ROOT, engineCheckout } from "./engine_checkout.mjs";

const runner = join(engineCheckout(), "scripts", "web", "run_in_browser.mjs");
const result = spawnSync(process.execPath, [runner, ...process.argv.slice(2)], { cwd: ARENA_ROOT, stdio: "inherit" });
process.exit(result.status ?? 1);
