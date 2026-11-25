# Retreat Arena

A third-person arena shooter built with the [Retreat engine](https://github.com/RetreatSoftware/Retreat-Engine-3D)
and [Retreat modules](https://github.com/RetreatSoftware/Retreat-Modules). Early stage: the arena map
can be viewed and the health and match rules run; there are no playable characters yet.

See the map in a window:

```bash
cargo run
```

Run the check and write pictures to `target/snapshots/`:

```bash
cargo run -- --headless --commands scripts/verify/arena_map.cmds
```

Check the health and match rules:

```bash
cargo run -- --headless --no-render --commands scripts/verify/simulation/health_match.cmds
```
