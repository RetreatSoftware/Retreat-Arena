#!/usr/bin/env node
// Proves every combination of template.json: makes a copy of the game without the modules a
// combination leaves out, then checks it (format, lint, tests with the full match, WebAssembly check)
// and runs the verify scripts it has the modules for. Run from the repository root:
//
//   node scripts/combinations/check.mjs                      all combinations
//   node scripts/combinations/check.mjs rockets-off_items-none   only those named
//   node scripts/combinations/check.mjs --list               what would be checked, nothing built
//   node scripts/combinations/check.mjs <name> --tests-only   format, lint and tests only (CI)
//
// Copies are made in the system's temporary directory (or --work <folder>) and share this
// repository's target/. Snapshots of a combination: target/combinations/<name>/. Afterwards
// target/debug/ holds the last combination's game: `cargo build` before running the game from here.
import { spawnSync } from "node:child_process";
import { cpSync, existsSync, mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
const template = JSON.parse(readFileSync(join(root, "template.json"), "utf8"));
const rules = template.leave_out_a_module;

const args = process.argv.slice(2);
const listOnly = args.includes("--list");
const testsOnly = args.includes("--tests-only");
const workIndex = args.indexOf("--work");
const work = workIndex >= 0 ? resolve(args[workIndex + 1]) : join(tmpdir(), "retreat-arena-combinations");
// Everything else is a combination's name.
const wanted = args.filter((arg, index) => !arg.startsWith("--") && (workIndex < 0 || index !== workIndex + 1));

function fail(message) {
  console.error(`combinations: FAIL: ${message}`);
  process.exit(1);
}

/** Every choice of every option: `{ name, modules }`. */
function combinations() {
  let all = [{ parts: [], modules: [] }];
  for (const option of template.options) {
    all = all.flatMap((before) =>
      option.choices.map((choice) => ({
        parts: [...before.parts, `${option.id}-${choice.id}`],
        modules: [...before.modules, ...choice.modules],
      })));
  }
  return all.map(({ parts, modules }) => ({ name: parts.join("_"), modules }));
}

/** The verify scripts, with what their headers say. */
function scripts() {
  const base = join(root, template.scripts.directory);
  return readdirSync(base, { recursive: true })
    .map((path) => path.replaceAll("\\", "/"))
    .filter((path) => path.endsWith(".cmds"))
    .sort()
    .map((path) => {
      const lines = readFileSync(join(base, path), "utf8").split(/\r?\n/);
      const listed = (header) => lines.find((line) => line.startsWith(header))?.slice(header.length).split(" ").filter((id) => id && id !== "nothing");
      const needs = listed(template.scripts.needs_header);
      if (!needs) fail(`${path} has no \`${template.scripts.needs_header}\` line: say which optional modules it needs, or \`nothing\``);
      const without = listed(template.scripts.without_header) ?? [];
      for (const id of [...needs, ...without]) {
        if (!template.optional_modules.some((module) => module.id === id)) fail(`${path} names \`${id}\`, which is no optional module of template.json`);
      }
      return { path, needs, without };
    });
}

const applies = (script, modules) =>
  script.needs.every((id) => modules.includes(id)) && !script.without.some((id) => modules.includes(id));

/** Removes whole lines, each of which must be there. */
function withoutLines(path, isLeftOut, expected) {
  const lines = readFileSync(path, "utf8").split("\n");
  const kept = lines.filter((line) => !isLeftOut(line.replace(/\r$/, "")));
  if (lines.length - kept.length !== expected) fail(`${path}: expected to leave out ${expected} line(s), found ${lines.length - kept.length}`);
  writeFileSync(path, kept.join("\n"));
}

/** The game with only `modules`, in `folder`. */
function makeGame(folder, modules) {
  rmSync(folder, { recursive: true, force: true });
  mkdirSync(folder, { recursive: true });
  for (const entry of ["Cargo.toml", "Cargo.lock", "template.json", "src", "assets", "scripts", ".cargo"]) {
    if (existsSync(join(root, entry))) cpSync(join(root, entry), join(folder, entry), { recursive: true });
  }
  for (const module of template.optional_modules) {
    if (modules.includes(module.id)) continue;
    rmSync(join(folder, module.directory), { recursive: true });
    const listed = rules.module_list_lines.map((line) => line.replace("{id}", module.id));
    withoutLines(join(folder, rules.module_list_file), (line) => listed.includes(line), listed.length);
    const dependency = rules.dependency_line_start.replace("{crate}", module.crate);
    withoutLines(join(folder, rules.dependency_file), (line) => line.startsWith(dependency), 1);
  }
}

function run(folder, what, command, commandArgs) {
  const result = spawnSync(command, commandArgs, {
    cwd: folder,
    env: { ...process.env, CARGO_TARGET_DIR: join(root, "target") },
    encoding: "utf8",
    maxBuffer: 1 << 28,
  });
  if (result.status !== 0) {
    const output = `${result.stdout ?? ""}${result.stderr ?? ""}${result.error ?? ""}`;
    fail(`${what} failed in ${folder}\n${output.split("\n").slice(-60).join("\n")}`);
  }
  console.log(`combinations:   ok  ${what}`);
}

const allScripts = scripts();
for (const module of template.optional_modules) {
  for (const needed of module.requires) {
    for (const { name, modules } of combinations()) {
      if (modules.includes(module.id) && !modules.includes(needed)) fail(`${name} offers \`${module.id}\` without \`${needed}\`, which it requires`);
    }
  }
}
const known = combinations();
const unknown = wanted.filter((name) => !known.some((combination) => combination.name === name));
if (unknown.length) fail(`unknown combination(s) ${unknown.join(", ")}; known: ${known.map((c) => c.name).join(", ")}`);

for (const { name, modules } of known.filter((c) => !wanted.length || wanted.includes(c.name))) {
  const mine = allScripts.filter((script) => applies(script, modules));
  console.log(`combinations: ${name}: modules [${modules.join(", ")}], ${mine.length} scripts`);
  // The network scripts are run by scripts/verify/network/run.mjs, several processes at once.
  const alone = mine.filter((script) => !script.path.startsWith("network/"));
  if (listOnly) {
    for (const script of mine) console.log(`combinations:   ${script.path}`);
    continue;
  }
  const folder = join(work, name);
  makeGame(folder, modules);
  run(folder, "cargo fmt --check", "cargo", ["fmt", "--check"]);
  run(folder, "cargo clippy", "cargo", ["clippy", "--all-targets", "--", "-D", "warnings"]);
  run(folder, "cargo test", "cargo", ["test"]);
  if (testsOnly) {
    run(folder, "cargo clean of the copy", "cargo", ["clean", "-p", "retreat_arena"]);
    rmSync(folder, { recursive: true });
    continue;
  }
  run(folder, "cargo check for WebAssembly", "cargo", ["check", "--target", "wasm32-unknown-unknown"]);
  run(folder, "cargo build", "cargo", ["build"]);
  for (const script of alone) {
    const mode = script.path.startsWith("simulation/") ? ["--no-render"] : [];
    const path = `${template.scripts.directory}/${script.path}`;
    run(folder, path, "cargo", ["run", "-q", "--", "--headless", ...mode, "--commands", path]);
  }
  if (mine.some((script) => script.path.startsWith("network/"))) {
    run(folder, "network check", process.execPath, ["scripts/verify/network/run.mjs", "--software-renderer"]);
  }
  const pictures = join(root, "target", "combinations", name);
  rmSync(pictures, { recursive: true, force: true });
  cpSync(join(folder, "target", "snapshots"), pictures, { recursive: true });
  // The game's own build output (several hundred MB a copy); dependencies stay shared.
  run(folder, "cargo clean of the copy", "cargo", ["clean", "-p", "retreat_arena"]);
  rmSync(folder, { recursive: true });
}
console.log(listOnly ? "combinations: listed" : "combinations: PASS");
