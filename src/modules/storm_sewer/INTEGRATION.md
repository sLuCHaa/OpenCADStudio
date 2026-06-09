# Storm Sewer module — integration plan

The ribbon tab (`mod.rs`) and the engine bridge (`analysis.rs`) are in place.
What remains is wiring each `SS_*` ribbon command to a handler in the host
command system.

## Architecture

```
storm_sewer/mod.rs      ribbon tab  →  emits ModuleEvent::Command("SS_*")
        │
host command dispatch   (src/app/commands.rs / src/command/)
        │
storm_sewer/analysis.rs builds a stormsewer::Network and runs the engine
        │
stormsewer crate        Rational + Manning + HGL  →  Analysis + report text
```

The network/hydraulics data is carried as **XDATA on the drawing entities**
(`data.rs`), so the network round-trips to DWG/DXF and is analyzable directly:

| Network object | Entity | XDATA record `STORMSEWER_*` |
|----------------|--------|------------------------------|
| Structure (inlet/junction/outfall) | CIRCLE at the point | `STRUCT`: [kind, invert, rim, area, C] |
| Pipe (link) | LINE between two structures | `PIPE`: [diameter, n, from-handle, to-handle] |

Connectivity is by entity handle: a pipe stores the handles of the two
structures it connects, so `data::network_from_entities` rebuilds the
`stormsewer::Network` (nodes N1, N2, … by encounter order) from the drawing.

## Command handlers — status

All `SS_*` commands are wired in `src/app/commands.rs::dispatch_command`:

| Command   | Status | Action |
|-----------|--------|--------|
| `SS_INLET`/`SS_JUNCTION`/`SS_OUTFALL` | ✅ done | `PlaceStructure` — enter invert/rim/area/C, then click the location; commits an XDATA-tagged circle |
| `SS_PIPE` | ✅ done | `PlacePipe` — enter diameter/n, then click the START and END structures; commits an XDATA-tagged line (endpoints at the clicks, connectivity by structure handle) |
| `SS_ANALYZE` | ✅ done | rebuild the network from drawn entities → run engine → add flow/HGL labels + print report |
| `SS_REPORT` | ✅ done | rebuild from drawn entities → print `report::format_analysis()` |
| `SS_PROFILE` | ✅ done | rebuild from drawn entities → draw the HGL/invert/ground long-section |

**Interaction order matters.** Both commands collect typed values FIRST (the
command line is focused via `focus_cmd_input()` at dispatch and stays focused
through text input), then take the viewport click LAST and commit from it. A
point-pick result (`NeedPoint`) does not re-focus the command line, so a
click-then-type flow would lose focus and route Enter to `on_enter` → cancel.

The `.ssn` file path is retained in `analysis.rs` (`analyze_text` etc.) for
file-based workflows and tests, but the ribbon commands now operate on the
**drawn network**.

Snapping note: pipe endpoints land at the click points (where you click on each
structure). Connectivity is exact (by handle) and analysis uses the structure
centers, so the analysis is correct; a future refinement could redraw pipes
exactly center-to-center.

## Remaining enhancements

- **Rainfall parameters UI** — `SS_ANALYZE` uses a default IDF (`60/(t+10)^0.8`)
  and free outfall; add an `SS_PARAMS` command to set IDF / tailwater / min-Tc.
- **Edit command** — `SS_EDIT` to change a placed structure/pipe's values.
- **Surcharge styling** — recolor surcharged pipes / flag flooded structures.
- **Persistence check** — verify the StormSewer XDATA round-trips through
  DWG/DXF save+reload (acadrust supports XDATA; confirm end-to-end).

## Build

`build.rs` auto-discovers this directory (`storm_sewer/` → `StormSewerModule`)
and regenerates `src/modules/registry.rs`, so the tab appears on `cargo build`.
