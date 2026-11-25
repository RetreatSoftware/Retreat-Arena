// Network check: the game's server and two clients as real processes talking WebTransport on
// localhost, each running its script in real time. Client A moves; client B renders, watches, then
// shoots A dead and sees it respawn.
//
//   cargo build
//   node scripts/verify/network/run.mjs [--software-renderer]
//   node scripts/web/build.mjs; node scripts/verify/network/run.mjs --browser   (client B in headless Chrome)
//
// Passes when every process exits 0, A came back away from where it was shot, B stayed where the server
// put it, and all three agree on A's character (position and every synced component). Snapshots: target/snapshots/ (browser:
// target/snapshots/web/).
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "..", "..", "..");
const exe = join(root, "target", "debug", process.platform === "win32" ? "retreat_arena.exe" : "retreat_arena");
const browser = process.argv.includes("--browser");
const softwareRenderer = process.argv.includes("--software-renderer") ? ["--software-renderer"] : [];
const TIMEOUT_MS = 900_000;
/** How far apart two machines may see a body. */
const TOLERANCE = 0.05;
/** Where the server puts player 0 to be shot. */
const SHOT_A = [-0.15, 0.9, 17.37];
/** Where the server puts player 1, who then stays there. */
const PLACE_B = [3, 0.9, 18];
const processes = [];

function fail(message) {
  const tails = processes.map((p) => `----- ${p.name} (exit ${p.exited}) -----\n${p.output.split("\n").slice(-40).join("\n")}`);
  const report = `${message}\n${tails.join("\n")}`;
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
const server = start("server", exe, ["--headless", "--no-render", "--real-time", "--server", "0", "--commands", script("server")]);
const listening = await waitFor("the server to listen", () => server.output.match(/listening on UDP port (\d+) certificate-hash ([0-9a-f]{64})/));
const url = `https://127.0.0.1:${listening[1]}`;
const connect = ["--connect", url, "--cert-hash", listening[2]];
const a = start("client a", exe, ["--headless", "--no-render", "--real-time", ...connect, "--commands", script("client_a")]);
await waitFor("client a to join as player 0", () => /net client 0: Running/.test(a.output));
const b = browser
  ? start("browser client b", process.execPath, [
      join(root, "scripts", "web", "run_in_browser.mjs"),
      script("client_b"),
      "--timeout", String(TIMEOUT_MS / 1000),
      "--query", `connect=${encodeURIComponent(url)}&cert_hash=${listening[2]}&real_time=1`,
    ])
  : start("client b", exe, ["--headless", "--real-time", ...softwareRenderer, ...connect, "--commands", script("client_b")]);
await waitFor("every process to finish", () => processes.every((p) => p.exited === 0));

const seenByServer = positions(server).get("player_0");
if (!seenByServer) fail("the server printed no position for player_0");
if (distance(seenByServer, SHOT_A) < 5) fail(`player_0 did not respawn at a spawn point: ${seenByServer}`);
for (const p of [a, b]) {
  const seen = positions(p).get("player_0");
  if (!seen || distance(seen, seenByServer) > TOLERANCE) fail(`${p.name} sees player_0 at ${seen}, the server at ${seenByServer}`);
}
const idle = positions(b).get("player_1");
if (!idle || distance(idle, PLACE_B) > TOLERANCE) fail(`player_1 should be at ${PLACE_B}, client b sees ${idle}`);
const states = JSON.stringify(statesOfA(server));
if (!states.includes("movement.state")) fail(`the server printed no movement state: ${states}`);
for (const p of [a, b]) {
  if (JSON.stringify(statesOfA(p)) !== states) fail(`synced state of player_0 differs on ${p.name}: ${JSON.stringify(statesOfA(p))} vs ${states}`);
}
console.log(`network check: player_0 at ${seenByServer.join(" ")} on every process; ${statesOfA(server).length} synced values agree`);
for (const p of processes) console.log(`network check: ${p.name}: ${p.output.match(/net-stats (role=.*)/)?.[1]?.trim() ?? "no net-stats"}`);
console.log("network check: PASS");
