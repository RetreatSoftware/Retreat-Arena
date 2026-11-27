#!/usr/bin/env node
// Packages the game for players (package plan P2, decided 2026-11-27): one folder, by default dist/,
// that runs on another Windows PC without Rust, Node or this project.
//
//   node scripts/package.mjs [--out <folder>]
//
// The folder holds:
//   web/                   the browser game, release build: size-optimized, wasm-opt, and a Brotli copy
//                          (.br) of each file that compresses, which the server sends to browsers
//   retreat_arena.exe      the game server, release build, C runtime linked in (no Visual C++
//                          redistributable needed)
//   start-server.bat       starts the server, which serves web/ to browsers on the local network
//   README.md              how to start it and what players see
//   retreat-package.json   what this is and its sizes; marks the folder as a package, so a later run
//                          may replace it
// A folder that exists must be empty or such a package; anything else is refused and left alone.
// Windows only for now. Takes long: both builds are release builds (the development builds that
// `cargo run` and play-lan.bat use are not touched).
//
// Needs: rustup targets wasm32-unknown-unknown and x86_64-pc-windows-msvc, wasm-bindgen-cli at the
// version of the wasm-bindgen crate in Cargo.lock, and wasm-opt (Binaryen). What is missing is listed
// with the commands that install it, before anything is built.
import { spawnSync } from "node:child_process";
import { cpSync, existsSync, mkdirSync, readFileSync, readdirSync, renameSync, rmSync, writeFileSync } from "node:fs";
import { basename, dirname, join, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { ARENA_ROOT, engineCheckout } from "./web/engine_checkout.mjs";

const BIN = "retreat_arena";
const TITLE = "Retreat Arena";
const SERVER_TARGET = "x86_64-pc-windows-msvc";
const WEB_TARGET = "wasm32-unknown-unknown";
const PACKAGE_RECORD = "retreat-package.json";
const DEFAULT_PORT = 5000;
const DEFAULT_WEB_PORT = 8443;
const USAGE = "usage: node scripts/package.mjs [--out <folder>]";

function fail(message) {
  console.error(`package: ${message}`);
  process.exit(1);
}

function parseArguments(argv) {
  const options = { out: join(ARENA_ROOT, "dist") };
  for (let index = 0; index < argv.length; index += 2) {
    if (argv[index] !== "--out" || argv[index + 1] === undefined) fail(`bad argument \`${argv[index]}\`; ${USAGE}`);
    options.out = resolve(argv[index + 1]);
  }
  return options;
}

/** What `program` prints, or null when it does not run or fails. */
function answer(program, args) {
  const result = spawnSync(program, args, { encoding: "utf8" });
  return result.status === 0 ? result.stdout : null;
}

/** Each missing tool with the command that installs it. */
function missingTools() {
  const lock = readFileSync(join(ARENA_ROOT, "Cargo.lock"), "utf8");
  const wantedBindgen = lock.match(/name = "wasm-bindgen"\r?\nversion = "([^"]+)"/)?.[1];
  const missing = [];
  const targets = answer("rustup", ["target", "list", "--installed"]);
  for (const target of [WEB_TARGET, SERVER_TARGET]) {
    if (targets !== null && !targets.split(/\r?\n/).includes(target)) missing.push(`rustup target add ${target}`);
  }
  const bindgen = answer("wasm-bindgen", ["--version"])?.split(/\s+/)[1];
  if (wantedBindgen && bindgen !== wantedBindgen) {
    missing.push(`cargo install wasm-bindgen-cli --version ${wantedBindgen} --locked`);
  }
  if (answer("wasm-opt", ["--version"]) === null) missing.push("cargo install wasm-opt --locked");
  return missing;
}

/** Refuses a folder that holds something other than an earlier package. */
function checkOut(out) {
  const root = resolve(ARENA_ROOT);
  if ((root + "\\").startsWith(out + "\\") || (root + "/").startsWith(out + "/")) {
    fail(`--out \`${out}\` holds the project itself; choose a folder of its own, such as dist`);
  }
  if (!existsSync(out)) return;
  const entries = readdirSync(out);
  if (entries.length > 0 && !entries.includes(PACKAGE_RECORD)) {
    fail(`${out} already holds other files; choose an empty or new folder (a package is replaced only when it has ${PACKAGE_RECORD})`);
  }
}

function run(command, args, env = process.env) {
  console.log(`package: ${command} ${args.join(" ")}`);
  const result = spawnSync(command, args, { cwd: ARENA_ROOT, stdio: "inherit", env });
  if (result.error) fail(`cannot run \`${command}\` (${result.error.message}); install it`);
  if (result.status !== 0) fail(`\`${command} ${args.join(" ")}\` failed; its messages are above`);
}

function startServerBat() {
  return `@echo off
rem Starts the ${TITLE} game server. It also serves the game's page to browsers on this network:
rem players open the address it prints in Chrome or Edge. Close this window or press Ctrl+C to stop.
rem Other ports: set RETREAT_PORT (game, UDP) or RETREAT_WEB_PORT (page, TCP) before starting.
rem More options (for example --lan-address 192.168.1.20) can follow start-server.bat.
setlocal
cd /d "%~dp0"
if not defined RETREAT_PORT set RETREAT_PORT=${DEFAULT_PORT}
if not defined RETREAT_WEB_PORT set RETREAT_WEB_PORT=${DEFAULT_WEB_PORT}
rem The server reads the game's files (the map) from the web folder.
set "BEVY_ASSET_ROOT=%~dp0web"
"%~dp0${BIN}.exe" --headless --no-render --server %RETREAT_PORT% --serve-web web --web-port %RETREAT_WEB_PORT% %*
if errorlevel 1 pause
`.replaceAll("\n", "\r\n");
}

function readme(sizes) {
  return `# ${TITLE}

The packaged game: a server for Windows and the game's page, which players open in Chrome or Edge on
the same local network. Nothing else needs to be installed on this computer or on the players'.

## Start

Double-click \`start-server.bat\` (or run \`.\\start-server.bat\` in PowerShell). It prints the
address to open, for example \`https://192.168.1.20:${DEFAULT_WEB_PORT}/\`.

1. If Windows asks about the firewall, allow the app on **Private networks** (players need TCP port
   ${DEFAULT_WEB_PORT} and UDP port ${DEFAULT_PORT}).
2. Each player opens the printed address in Chrome or Edge. On this computer
   \`https://localhost:${DEFAULT_WEB_PORT}/\` also works.
3. The browser warns that the certificate is not trusted (the server made it itself): choose
   **Advanced**, then **Proceed**. The warning comes back after every server start.

The page joins the game by itself. Close the window or press Ctrl+C to stop the server.

Other ports: \`set RETREAT_PORT=5001\` and \`set RETREAT_WEB_PORT=9443\` in a terminal, then
\`start-server.bat\` from it. A wrong printed address (a VPN): \`start-server.bat --lan-address 192.168.1.20\`.

## Size

A player downloads about ${(sizes.downloadBytes / 1e6).toFixed(1)} MB on each visit (the browser keeps nothing because of the
certificate warning); unpacked it is ${(sizes.uncompressedBytes / 1e6).toFixed(1)} MB.

## Limits

Local network only: no internet hosting, no real certificate. The server runs on Windows.
`;
}

const { out } = parseArguments(process.argv.slice(2));
if (process.platform !== "win32") fail("packaging makes a Windows server and runs on Windows only for now");
checkOut(out);
const missing = missingTools();
if (missing.length > 0) {
  fail(`packaging needs more tools. Run these in a terminal, then package again:\n${missing.join("\n")}`);
}

const engine = engineCheckout();
const { measureWebFolder } = await import(pathToFileURL(join(engine, "scripts", "web", "web_sizes.mjs")).href);
const buildFolder = process.env.CARGO_TARGET_DIR ?? join(ARENA_ROOT, "target");
const webBuild = join(buildFolder, "package", "web");

console.log("package: 1 of 3: the browser game (release build; the first one takes long)");
run(process.execPath, [join(engine, "scripts", "web", "build_web.mjs"), "--bin", BIN, "--title", TITLE, "--release", "--no-scripts", "--out", webBuild]);

console.log("package: 2 of 3: the server (release build)");
// The C runtime is linked in, so the server needs no Visual C++ redistributable. Set for this
// target only, so build scripts and macros are built as usual.
const serverEnv = { ...process.env, CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS: "-C target-feature=+crt-static" };
run("cargo", ["build", "--locked", "--release", "--bin", BIN, "--target", SERVER_TARGET], serverEnv);

console.log("package: 3 of 3: the folder");
const partial = join(dirname(out), `${basename(out)}.partial`);
rmSync(partial, { recursive: true, force: true });
mkdirSync(partial, { recursive: true });
cpSync(webBuild, join(partial, "web"), { recursive: true });
cpSync(join(buildFolder, SERVER_TARGET, "release", `${BIN}.exe`), join(partial, `${BIN}.exe`));
const sizes = measureWebFolder(join(partial, "web"));
writeFileSync(join(partial, "start-server.bat"), startServerBat());
writeFileSync(join(partial, "README.md"), readme(sizes));
const record = { game: TITLE, server: `${BIN}.exe`, sizes };
writeFileSync(join(partial, PACKAGE_RECORD), `${JSON.stringify(record, null, 2)}\n`);
try {
  rmSync(out, { recursive: true, force: true });
  renameSync(partial, out);
} catch (error) {
  fail(`cannot replace ${out} (${error.code}); stop a server that runs from it and close what has its files open, then package again`);
}
// Forge's window reads these two lines (`package: folder: `, `package: done: `).
console.log(`package: folder: ${out}`);
console.log(
  `package: done: players download ${(sizes.downloadBytes / 1e6).toFixed(1)} MB (${(sizes.uncompressedBytes / 1e6).toFixed(1)} MB unpacked)`,
);
