# Open CAD Studio

<img width="1920" height="940" alt="resim" src="https://github.com/user-attachments/assets/10635ad0-454b-4c87-935f-1a3a46f24ccb" />

A CAD application for 2D drafting and 3D modeling, built with Rust. Reads and writes DWG and DXF files natively.

## Features

### File Formats
- **DWG** read/write (R13 through R2018)
- **DXF** read/write (R13 through R2018)
- **STL** export (`STLOUT` / `EXPORTSTL`)
- **STEP AP203** export (`STEPOUT`)
- **OBJ** import (`IMPORTOBJ`)
- **PDF** export (plot layouts to PDF)
- **WBLOCK** — write selected entities or a block to an external file
- **XREF** — attach, reload, and auto-resolve external references

### 2D Drafting
| Command | Description |
|---------|-------------|
| `LINE`, `PLINE`, `RECTANG`, `POLYGON` | Basic geometry |
| `CIRCLE`, `ARC`, `ELLIPSE`, `SPLINE` | Curves |
| `HATCH`, `HATCHEDIT` | Hatch fills with pattern, scale, angle editing |
| `OFFSET`, `TRIM`, `EXTEND`, `FILLET` | Modify geometry (supports lines, arcs, ellipses, polylines, splines) |
| `BREAK`, `STRETCH`, `LENGTHEN` | Shape editing |
| `ARRAY`, `MIRROR`, `MOVE`, `COPY`, `ROTATE`, `SCALE` | Transformations |
| `EXPLODE` | Explode blocks, dimensions, polylines, mlines |
| `DDEDIT` | Double-click text editing |
| `MASSPROP` | Area, perimeter, centroid of selected entities |

### 3D Modeling
| Command | Description |
|---------|-------------|
| `BOX`, `SPHERE`, `CYLINDER` | Solid primitives |
| `EXTRUDE`, `REVOLVE` | Profile-based solids |
| `LOFT` | Ruled-surface loft through cross-sections |
| `SWEEP` | Sweep a profile along a path |
| `ARRAY3D` | 3D array |
| ACIS tessellation | Renders `3DSOLID`, `REGION`, and `BODY` entities |

### Annotations & Dimensions
- **Dimensions**: Linear, Aligned, Angular, Radial, Diameter, Ordinate — with full `DIMSTYLE` support (`DIMASZ`, `DIMSCALE`, `DIMEXO`, `DIMEXE`, and more)
- **Text**: `MTEXT`, `TEXT`, `DTEXT` with font browser (`STYLE DIALOG`)
- **Leaders**: `MLEADER` with straight and spline path types; `MLEADERSTYLE` manager
- **Tolerances**: GD&T feature control frames
- **Tables**: `TABLE` entity render; `TABLESTYLE` manager
- **MLine**: `MLINE` entity with `MLSTYLE` manager and `EXPLODE` support

### Paper Space & Layouts
- Multi-tab layout system with model space and unlimited paper space tabs
- **Viewport projection**: Model content correctly projected into paper-space viewport rectangles
- **Camera persistence**: View position and zoom saved per layout; restored on file open and tab switch
- **Correct paper size**: Physical paper dimensions read from embedded PlotSettings (not drawing limits)
- Inline MSPACE overlay — enter a viewport with double-click; edit model entities in place
- `VPORTS` — preset viewport configurations (single, 2H, 2V, 4-way)
- `LAYOUTMANAGER` / `LAYOUTPANEL` — GUI layout manager
- `PLOTSTYLEPANEL` / `STYLESMANAGER` — plot style table editor (CTB/STB)
- `PRINT` — send layout to system printer

### Blocks & References
- `INSERT` with attribute prompting (`ATTREQ`)
- `ATTEDIT` — edit block attribute values interactively
- `REFEDIT` / `REFCLOSE` — in-place block reference editing
- `XREF` — attach, reload, and resolve external DWG/DXF references
- `DATAEXTRACTION` — export entity property data to CSV

### Snapping & Precision
- Object snaps: Endpoint, Midpoint, Center, Node, Quadrant, Intersection, Perpendicular, Tangent, Nearest, Insertion, and more
- Ellipse arc endpoints, LWPolyline arc midpoints, Hatch boundary points
- **Object Snap Tracking** (`OTRACK` / `F11`)
- **Polar Tracking** with configurable angle increment
- **Dynamic Input** overlay (`DYNMODE` / `F12`)
- Grid snap with adaptive spacing
- Command history navigation (↑ / ↓)

### Rendering
- GPU-accelerated via WebGPU (wgpu)
- 4× MSAA anti-aliasing
- Orthographic and perspective camera
- ViewCube with face/edge/corner snapping
- **Wide polylines**: LWPolyline and Polyline2D filled strokes
- **Raster images**: GPU-textured quad pipeline (`IMAGE` command)
- **Wipeout**: Solid fill masking
- **Complex linetypes**: Text and shape elements rendered in linetype patterns
- White/black entity colors adapt to background luminance
- Per-viewport background color (`BACKGROUND`)
- Visual style selector (Wireframe, Shaded, etc.)
- X-ray ghost pass for selected wires occluded by geometry

### UI
- Modular ribbon interface — Home, Insert, Annotate, View, Manage, Layout
- Command line with autocomplete and history
- Layer Manager with per-viewport freeze columns
- Properties panel
- `COLORSCHEME` — runtime theme switching
- `SHORTCUTS` — keyboard shortcuts panel
- `SPLINEDIT` — close, open, reverse spline control points
- UCS icon with 3D foreshortening and axis labels

## Installation

### Linux (AppImage)

Download `OpenCADStudio-*-linux-x86_64.AppImage` from the [latest release](https://github.com/HakanSeven12/OpenCADStudio/releases/latest), then:

```bash
chmod +x OpenCADStudio-*-linux-x86_64.AppImage
./OpenCADStudio-*-linux-x86_64.AppImage
```

No installation required — runs directly on any modern Linux distribution.

### Windows

Download `OpenCADStudio-*-windows-x86_64.exe` from the [latest release](https://github.com/HakanSeven12/OpenCADStudio/releases/latest) and run it directly. Windows SmartScreen may show "Windows protected your PC" because the binary is not yet code-signed — click **More info → Run anyway**.

### macOS (Apple Silicon)

Download `OpenCADStudio-*-macos-arm64.dmg` from the [latest release](https://github.com/HakanSeven12/OpenCADStudio/releases/latest) — Apple Silicon (M-series) only; Intel macOS isn't built.

Open the .dmg and drag `OpenCADStudio.app` to `/Applications`. The first launch may be blocked by Gatekeeper because the app is not notarised — right-click `OpenCADStudio.app → Open` and confirm, or run once via `xattr -dr com.apple.quarantine /Applications/OpenCADStudio.app`.

### Build from Source

Requirements: Rust 1.75+

```bash
git clone https://github.com/HakanSeven12/OpenCADStudio.git
cd OpenCADStudio
cargo build --release --bin OpenCADStudio
./target/release/OpenCADStudio
```

## License

GPL-3.0-only — see [LICENSE](LICENSE)
