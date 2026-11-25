// Where the engine's source is: the checkout Cargo uses for the pinned `retreat_engine` commit. The
// browser page and the browser script runner are the engine's own files, read from there, so this
// game keeps no copy that could drift from the engine it builds against.
import { execFileSync } from "node:child_process";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

/** This repository's root; every script here runs from it. */
export const ARENA_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");

export function engineCheckout() {
  const metadata = JSON.parse(
    execFileSync("cargo", ["metadata", "--format-version", "1", "--locked"], {
      cwd: ARENA_ROOT,
      maxBuffer: 256 * 1024 * 1024,
    }),
  );
  const engine = metadata.packages.find((found) => found.name === "retreat_engine");
  if (!engine) throw new Error("cargo metadata lists no package `retreat_engine`");
  // <checkout>/crates/retreat_engine/Cargo.toml
  return join(dirname(engine.manifest_path), "..", "..");
}
