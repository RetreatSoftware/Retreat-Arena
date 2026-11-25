# Retreat Arena

A third-person arena shooter built with the [Retreat engine](https://github.com/RetreatSoftware/Retreat-Engine-3D)
and [Retreat modules](https://github.com/RetreatSoftware/Retreat-Modules). Early stage: the arena map
can be viewed; there are no players yet.

See the map in a window:

```bash
cargo run
```

Run the check and write pictures to `target/snapshots/`:

```bash
cargo run -- --headless --commands scripts/verify/arena_map.cmds
```
