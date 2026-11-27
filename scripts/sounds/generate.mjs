#!/usr/bin/env node
// Writes the game's sounds into assets/sounds/ as 16-bit mono WAV files. Every sound is made here
// from sine waves and seeded noise: nothing is recorded or downloaded, and a run always writes the
// same bytes.
//
//   node scripts/sounds/generate.mjs           writes the files
//   node scripts/sounds/generate.mjs --check   fails if a committed file differs (CI)
//
// A new sound is one entry in SOUNDS; src/arena_game.rs gives it a volume and the module that plays
// it. Peaks stay well below full scale so several at once do not clip.
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const RATE = 22050;
const PEAK = 0.7;
const OUT = join(resolve(dirname(fileURLToPath(import.meta.url)), "..", ".."), "assets", "sounds");
const TAU = 2 * Math.PI;

/** Noise from -1 to 1 that is the same in every run. */
function noise(seed) {
  let state = seed >>> 0 || 1;
  return () => {
    state ^= state << 13;
    state >>>= 0;
    state ^= state >>> 17;
    state ^= state << 5;
    state >>>= 0;
    return (state / 0xffffffff) * 2 - 1;
  };
}

/** A one-pole low pass; `cutoff` in Hz may change from sample to sample. */
function lowPass() {
  let value = 0;
  return (input, cutoff) => {
    value += (1 - Math.exp((-TAU * cutoff) / RATE)) * (input - value);
    return value;
  };
}

/** A sine whose frequency may change from sample to sample. */
function sine() {
  let phase = 0;
  return (frequency) => {
    phase += (TAU * frequency) / RATE;
    return Math.sin(phase);
  };
}

/** `from` at 0 to `to` at 1, exponentially. */
const glide = (from, to, part) => from * (to / from) ** Math.min(1, Math.max(0, part));
const decay = (time, constant) => Math.exp(-time / constant);

/** Notes one after another: `[frequency, seconds]`, each with a soft start and a fading end. */
function melody(notes) {
  const seconds = notes.reduce((sum, [, length]) => sum + length, 0);
  const tone = sine();
  const over = sine();
  return [seconds, (time) => {
    let start = 0;
    for (const [frequency, length] of notes) {
      if (time < start + length) {
        const local = time - start;
        const level = Math.min(1, local / 0.008) * decay(local, length * 0.6);
        return level * (tone(frequency) + 0.25 * over(frequency * 2));
      }
      start += length;
    }
    return 0;
  }];
}

/** Name to `[seconds, sample(time)]`; each sound builds its own oscillators. */
const SOUNDS = {
  rifle_shot: () => {
    const hiss = noise(11);
    const body = lowPass();
    const thump = sine();
    return [0.16, (t) => body(hiss(), glide(6000, 700, t / 0.12)) * decay(t, 0.03) + 0.6 * thump(glide(220, 70, t / 0.08)) * decay(t, 0.04)];
  },
  rifle_reload: () => {
    const hiss = noise(23);
    const body = lowPass();
    // Two clicks: out and in.
    const click = (t, at) => (t >= at ? decay(t - at, 0.012) : 0);
    return [0.45, (t) => body(hiss(), 3500) * (click(t, 0.02) + click(t, 0.3))];
  },
  rocket_launch: () => {
    const hiss = noise(37);
    const body = lowPass();
    const low = sine();
    return [0.5, (t) => Math.min(1, t / 0.03) * decay(t, 0.18) * (body(hiss(), glide(900, 3500, t / 0.35)) + 0.5 * low(glide(110, 220, t / 0.4)))];
  },
  explosion: () => {
    const hiss = noise(41);
    const body = lowPass();
    const low = sine();
    return [1.1, (t) => 1.4 * body(hiss(), glide(2500, 90, t / 0.9)) * decay(t, 0.28) + 0.8 * low(glide(90, 35, t / 0.6)) * decay(t, 0.3)];
  },
  hit: () => melody([[1480, 0.04], [1980, 0.05]]),
  death: () => melody([[392, 0.14], [311, 0.14], [233, 0.3]]),
  respawn: () => melody([[330, 0.09], [494, 0.09], [659, 0.18]]),
  jump: () => {
    const tone = sine();
    return [0.14, (t) => tone(glide(220, 440, t / 0.12)) * Math.min(1, t / 0.01) * decay(t, 0.05)];
  },
  dash: () => {
    const hiss = noise(53);
    const body = lowPass();
    return [0.25, (t) => body(hiss(), glide(600, 5000, t / 0.2)) * Math.min(1, t / 0.04) * decay(t, 0.08)];
  },
  heal: () => melody([[523, 0.07], [659, 0.07], [784, 0.07], [1047, 0.16]]),
  pickup_health: () => melody([[659, 0.06], [988, 0.12]]),
  pickup_ammo: () => {
    const hiss = noise(67);
    const body = lowPass();
    // Two metallic ticks.
    const tick = (t, at) => (t >= at ? decay(t - at, 0.018) : 0);
    const ring = sine();
    return [0.2, (t) => (body(hiss(), 5000) + 0.5 * ring(1750)) * (tick(t, 0.0) + tick(t, 0.08))];
  },
  pickup_weapon: () => melody([[392, 0.07], [523, 0.07], [784, 0.07], [1047, 0.2]]),
  match_start: () => melody([[523, 0.14], [523, 0.14], [784, 0.35]]),
  match_end: () => melody([[784, 0.16], [659, 0.16], [523, 0.16], [392, 0.45]]),
};

/** The WAV file of one sound, its loudest sample at PEAK. */
function wav([seconds, sample]) {
  const count = Math.round(seconds * RATE);
  const values = Array.from({ length: count }, (_, index) => {
    // The last 5 ms fade out, so no file ends with a click.
    const fade = Math.min(1, (count - index) / (RATE * 0.005));
    return sample(index / RATE) * fade;
  });
  const loudest = values.reduce((most, value) => Math.max(most, Math.abs(value)), 0);
  const file = Buffer.alloc(44 + count * 2);
  file.write("RIFF", 0);
  file.writeUInt32LE(36 + count * 2, 4);
  file.write("WAVEfmt ", 8);
  file.writeUInt32LE(16, 16);
  file.writeUInt16LE(1, 20);
  file.writeUInt16LE(1, 22);
  file.writeUInt32LE(RATE, 24);
  file.writeUInt32LE(RATE * 2, 28);
  file.writeUInt16LE(2, 32);
  file.writeUInt16LE(16, 34);
  file.write("data", 36);
  file.writeUInt32LE(count * 2, 40);
  values.forEach((value, index) => file.writeInt16LE(Math.round((value / loudest) * PEAK * 32767), 44 + index * 2));
  return file;
}

const check = process.argv.includes("--check");
if (!check) mkdirSync(OUT, { recursive: true });
const stale = [];
for (const [name, build] of Object.entries(SOUNDS)) {
  const path = join(OUT, `${name}.wav`);
  const made = wav(build());
  if (!check) {
    writeFileSync(path, made);
    continue;
  }
  let committed;
  try {
    committed = readFileSync(path);
  } catch {
    committed = Buffer.alloc(0);
  }
  // One step of rounding may differ between processors' maths.
  const same = committed.length === made.length && committed.subarray(0, 44).equals(made.subarray(0, 44))
    && Array.from({ length: (made.length - 44) / 2 }, (_, i) => Math.abs(made.readInt16LE(44 + i * 2) - committed.readInt16LE(44 + i * 2))).every((difference) => difference <= 1);
  if (!same) stale.push(name);
}
if (stale.length) {
  console.error(`sounds: assets/sounds differs from the generator for: ${stale.join(", ")}; run \`node scripts/sounds/generate.mjs\` and commit the files`);
  process.exit(1);
}
console.log(`sounds: ${Object.keys(SOUNDS).length} ${check ? "checked" : "written"}`);
