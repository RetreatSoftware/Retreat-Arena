// Network check: the game's server and two clients as real processes talking WebTransport on
// localhost, each running its script in real time. Client A moves; client B renders, watches, then
// shoots A dead and sees it respawn.
//
//   cargo build
//   node scripts/verify/network/run.mjs [--software-renderer]
//   node scripts/web/build.mjs; node scripts/verify/network/run.mjs --browser   (client B in headless Chrome)
// --served-page (implies --browser) is the LAN game of play-lan.bat: the server also serves the web
// build over HTTPS (--serve-web) and client B opens that page at the network address the server prints,
// passes the certificate warning and joins by what the page is offered (no `connect` in its URL).
//
// Then client C joins the running match, the match ends with B as the winner and restarts.
// Passes when every process exits 0, A came back away from where it was shot, the restart moved B
// from where the server had put it, and server, A and B agree on A's character (position and every synced component). Snapshots: target/snapshots/ (browser:
// target/snapshots/web/).
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "..", "..", "..");
const exe = join(root, "target", "debug", process.platform === "win32" ? "retreat_arena.exe" : "retreat_arena");
const servedPage = process.argv.includes("--served-page");
const browser = servedPage || process.argv.includes("--browser");
const softwareRenderer = process.argv.includes("--software-renderer") ? ["--software-renderer"] : [];
const TIMEOUT_MS = 900_000;
/** Where the server puts player 1, until the match restarts. */
const PLACE_B = [3, 0.6, 18];
/** How far apart two machines may see a body. */
const TOLERANCE = 0.05;
/** Where the server puts player 0 to be shot. */
const SHOT_A = [-0.15, 0.6, 17.37];
const processes = [];

function fail(message) {
  // The errors first: a GitHub annotation keeps only about the first 4000 characters.
  const errors = processes.flatMap((p) =>
    p.output.split("\n").filter((line) => /ERROR|panicked|net server:/.test(line)).slice(0, 5).map((line) => `${p.name}: ${line.slice(0, 600)}`));
  const tails = processes.map((p) => `----- ${p.name} (exit ${p.exited}) -----\n${p.output.split("\n").slice(-40).join("\n")}`);
  const report = `${message}\n${errors.join("\n")}\n${tails.join("\n")}`;
  console.error(`network check: FAIL: ${report}`);
  // A GitHub annotation is readable without the raw log.
  if (process.env.GITHUB_ACTIONS) {
    console.log(`::error title=Network check failed::${report.replaceAll("%", "%25").replaceAll("\r", "%0D").replaceAll("\n", "%0A").slice(0, 60000)}`);
  }
  processes.forEach((p) => p.exited === null && p.child.kill());
  process.exit(1);
}

function start(name, command, args) {
  // The executable is started directly, so it is told where the game's `assets` folder is.
  const child = spawn(command, args, { cwd: root, env: { ...process.env, BEVY_ASSET_ROOT: root } });
  const started = { name, child, output: "", exited: null };
  const collect = (chunk) => {
    // Without the terminal color codes of the engine's log.
    started.output += chunk.toString().replace(/\u001b\[[0-9;]*m/g, "");
  };
  child.stdout.on("data", collect);
  child.stderr.on("data", collect);
  child.on("error", (error) => fail(`cannot start ${name} (${command}): ${error.message}`));
  child.on("exit", (code) => {
    started.exited = code;
    if (code !== 0) fail(`${name} exited with code ${code}`);
  });
  processes.push(started);
  return started;
}

async function waitFor(what, check) {
  const deadline = Date.now() + TIMEOUT_MS;
  for (;;) {
    const found = check();
    if (found) return found;
    if (Date.now() > deadline) fail(`timed out waiting for ${what}`);
    await new Promise((done) => setTimeout(done, 100));
  }
}

const positions = (p) =>
  new Map([...p.output.matchAll(/net-body (\S+) (-?[\d.]+) (-?[\d.]+) (-?[\d.]+)/g)].map((m) => [m[1], [Number(m[2]), Number(m[3]), Number(m[4])]]));
const statesOfA = (p) =>
  [...p.output.matchAll(/net-state (player_0 \S+ v\d+ [0-9a-f]*)/g)].map((m) => m[1]).sort();
const distance = (a, b) => Math.hypot(...a.map((value, axis) => value - b[axis]));

if (!existsSync(exe)) fail(`${exe} does not exist; run \`cargo build\` first`);
const script = (name) => `scripts/verify/network/${name}.cmds`;
const web = servedPage ? ["--serve-web", join("target", "web"), "--web-port", "0"] : [];
const server = start("server", exe, ["--headless", "--no-render", "--real-time", "--server", "0", ...web, "--commands", script("server")]);
const listening = await waitFor("the server to listen", () => server.output.match(/listening on UDP port (\d+) certificate-hash ([0-9a-f]{64})/));
let page;
if (servedPage) {
  // The network address when the machine has one, else this machine's own name (printed after it).
  await waitFor("the server to serve the page", () => server.output.includes("retreat-web: on this computer:"));
  page = (server.output.match(/retreat-web: players on this network open (\S+)/) ?? server.output.match(/retreat-web: on this computer: (\S+)/))[1];
  console.log(`network check: client b opens ${page}`);
}
const url = `https://127.0.0.1:${listening[1]}`;
const connect = ["--connect", url, "--cert-hash", listening[2]];
const a = start("client a", exe, ["--headless", "--no-render", "--real-time", ...connect, "--commands", script("client_a")]);
await waitFor("client a to join as player 0", () => /net client 0: Running/.test(a.output));
const b = browser
  ? start("browser client b", process.execPath, [
      join(root, "scripts", "web", "run_in_browser.mjs"),
      script("client_b"),
      "--timeout", String(TIMEOUT_MS / 1000),
      ...(servedPage
        ? ["--url", page, "--query", "real_time=1"]
        : ["--query", `connect=${encodeURIComponent(url)}&cert_hash=${listening[2]}&real_time=1`]),
    ])
  : start("client b", exe, ["--headless", "--real-time", ...softwareRenderer, ...connect, "--commands", script("client_b")]);
// Client C joins the match that is already running, once the server's script asks for it.
await waitFor("the server to ask for client c", () => /net server:/.test(server.output));
start("client c", exe, ["--headless", "--no-render", "--real-time", ...connect, "--commands", script("client_c")]);
await waitFor("every process to finish", () => processes.every((p) => p.exited === 0));

const seenByServer = positions(server).get("player_0");
if (!seenByServer) fail("the server printed no position for player_0");
if (distance(seenByServer, SHOT_A) < 5) fail(`player_0 did not respawn at a spawn point: ${seenByServer}`);
for (const p of [a, b]) {
  const seen = positions(p).get("player_0");
  if (!seen || distance(seen, seenByServer) > TOLERANCE) fail(`${p.name} sees player_0 at ${seen}, the server at ${seenByServer}`);
}
// The restart moved player 1 (who never walked) to a spawn point; the others print after it left.
const own = positions(b).get("player_1");
if (!own || distance(own, PLACE_B) < 5) fail(`the restart should have moved player_1 from ${PLACE_B}, client b sees ${own}`);
const states = JSON.stringify(statesOfA(server));
if (!states.includes("movement.state")) fail(`the server printed no movement state: ${states}`);
for (const p of [a, b]) {
  if (JSON.stringify(statesOfA(p)) !== states) fail(`synced state of player_0 differs on ${p.name}: ${JSON.stringify(statesOfA(p))} vs ${states}`);
}
console.log(`network check: player_0 at ${seenByServer.join(" ")} on every process; ${statesOfA(server).length} synced values agree`);
for (const p of processes) console.log(`network check: ${p.name}: ${p.output.match(/net-stats (role=.*)/)?.[1]?.trim() ?? "no net-stats"}`);
console.log("network check: PASS");
