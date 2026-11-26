#!/usr/bin/env node
// Builds the game for the browser into target/web/ with the engine's web build (its documentation
// lists what is written and what the build needs).
//
//   node scripts/web/build.mjs
import { spawnSync } from "node:child_process";
import { join } from "node:path";
import { ARENA_ROOT, engineCheckout } from "./engine_checkout.mjs";

const builder = join(engineCheckout(), "scripts", "web", "build_web.mjs");
const args = [builder, "--bin", "retreat_arena", "--title", "Retreat Arena"];
const result = spawnSync(process.execPath, args, { cwd: ARENA_ROOT, stdio: "inherit" });
process.exit(result.status ?? 1);
