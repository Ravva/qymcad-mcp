# qymcad-mcp

Headless recipes for [QymCAD](https://github.com/QymIs-Tech/QymCAD): an AI agent builds
and measures parametric documents without opening the window.

Related: [QymIs-Tech/QymCAD#6](https://github.com/QymIs-Tech/QymCAD/issues/6) (MCP-server).
The maintainer keeps MCP out of the main tree for now, so this lives as a separate
program. It is not built into QymCAD and does not drive its window: it links the same
crates (`qymcad-core`, `qymcad-kernel`, `qymcad-io`, `qymcad-testkit`) and builds through
the same core calls the interface delegates to, then rebuilds once with the real OCCT
kernel.

One process holds one document. Talk to it over stdin/stdout with JSON-RPC 2.0, one
request per line.

## Pinned upstream

Dependencies point at one upstream revision (`rev` in `Cargo.toml`). This is deliberate:
the `.qcad` format changes with no backward compatibility, so a document saved here opens
in the QymCAD build of the same revision. Moving the pin means re-running the tests.

## Requirements

- Rust 1.88 or newer.
- An installed OpenCASCADE with headers and libraries, exposed as:
  - `OCCT_INCLUDE_DIR` (e.g. `C:\occt\include\opencascade`)
  - `OCCT_LIB_DIR` (e.g. `C:\occt\lib`)
  - plus the OCCT `bin` directory on `PATH` at run time (Windows: `C:\occt\bin`).

## Running

```bash
cargo run -p qymcad-mcp
```

Point any MCP-compatible client at the binary as a stdio server, or pipe requests in:

```bash
echo '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}' | cargo run -q -p qymcad-mcp
```

## Tools

| Tool | Arguments | What it does |
| --- | --- | --- |
| `new_project` | — | Start an empty document with one part |
| `open_project` | `path` | Open a `.qcad` file with imports restored and a full rebuild |
| `save_project` | `path` | Save the document as `.qcad` |
| `apply_recipe` | `ops` | Build the ops in order, rebuild once, return measured bodies |
| `inspect` | — | Parts, bodies, sketches, nodes, with body numbers |
| `measure` | — | Measured numbers of every body without rebuilding |
| `export_stl` | `path` | Write display meshes as binary STL |
| `export_step` | `path` | Rebuild and write the live B-rep as STEP |

Recipe ops: `BoxOp {dx, dy, dz}`, `RectSketch {width, height}` (a rectangle on XY
centred at the origin), `Extrude {height}`, `Fillet {radius}` (every edge of the last
body). All sizes are finite positive millimetres; a refusal names its field
(`mcp-rect-size`, `mcp-extrude-without-sketch`, ...).

## Example

```json
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"apply_recipe","arguments":{"ops":[
  {"RectSketch":{"width":40.0,"height":30.0}},
  {"Extrude":{"height":10.0}}
]}}}
```

Answer: one body of 12000 mm^3 with its bounding box and triangle count. A saved
`.qcad` opens in QymCAD via File → Open and rebuilds parametrically.

## Notes

- Live B-rep shapes never leave the process; only numbers and files cross the wire.
- Error codes are machine-readable English keys, no interface strings.
- Checks: `cargo test` (a rect extruded holds its volume, refusals name their cause,
  a saved document opens back), `cargo fmt --check`, `cargo check --all-targets`.

## Licence

[AGPL-3.0-or-later](LICENSE), same as QymCAD.
