# Performance Optimization Plan

## Implemented

| Option | Description |
|--------|-------------|
| A | Wire tessellation cache — `Arc<Vec<WireModel>>` keyed by `geometry_epoch`; O(1) on navigation |
| B | GPU buffer cache — skip `upload_wires/hatches/images/meshes` when epoch unchanged |
| C | Rayon parallel tessellation — `tessellate_entity` free function + `par_iter()` in `wires_for_block()` |
| E | SortEntitiesTable scan cache — O(objects) linear scan replaced with O(1) HashMap lookup |

---

## Remaining per-frame CPU work (navigation path)

With Options A/B implemented, `build_primitive()` still runs every frame during navigation
(pan/zoom), even though the geometry epoch has not changed. The following work happens on every
frame and should be eliminated:

### 1. `ImageModel.pixels` cloned every frame — highest impact

`build_primitive()` calls `self.images.values().cloned().collect()`.
`ImageModel` contains `pixels: Vec<u8>` — raw RGBA pixel data for each image.
A single 2000×1500 image is ~12 MB. With three images, every mouse-move event copies ~36 MB.

### 2. `synced_hatch_models()` — full document scan + Insert explosion every frame

The inner loop `for entity in self.document.entities()` iterates every entity in the document,
and for each `Insert` calls `explode_from_document()` to find embedded hatches.
In drawings with many block references this is O(inserts × avg_block_size) per frame.

### 3. `meshes.values().cloned().collect()` — 3D geometry cloned every frame

`MeshModel` contains `verts: Vec<[f32;3]>` and `indices: Vec<u32>`.
A tessellated 3D solid can hold tens of thousands of triangles (several MB).
All mesh data is copied on every frame even during pure navigation.

### 4. `wipeout_models()` — full entity scan every frame

Iterates `self.document.entities()` to find `Wipeout` entities and rebuild their boundary
polygons. No caching; O(entities) on every frame.

---

## Option F — Cache hatch and wipeout models

**Status:** Done

Add epoch-keyed `Arc` caches for hatches and wipeouts, identical in structure to the wire cache:

```rust
hatch_cache:   RefCell<Option<(u64, Arc<Vec<HatchModel>>)>>,
wipeout_cache: RefCell<Option<(u64, Arc<Vec<HatchModel>>)>>,
```

Introduce `hatch_models_arc()` and `wipeout_models_arc()` helpers that return the cached `Arc`
on an epoch hit (O(1) refcount bump) and rebuild on a miss.

`build_primitive()` stores the `Arc` directly in `Primitive`, removing the per-frame
`synced_hatch_models()` / `wipeout_models()` calls on navigation frames.

`Primitive.hatches` and `Primitive.wipeout_hatches` change from `Vec<HatchModel>` to
`Arc<Vec<HatchModel>>`.

**Impact:** Eliminates O(inserts × block_size) work per frame in drawings with block references
containing hatch entities. Navigation becomes free for hatch-heavy files.

**Difficulty:** Easy. Same pattern as the wire cache.

---

## Option G — Arc-wrap image pixels

**Status:** Done

Change `ImageModel.pixels` from `Vec<u8>` to `Arc<Vec<u8>>`.

`ImageModel::clone()` then copies only the pointer (8 bytes) instead of megabytes of pixel data.
No other code needs to change — the GPU upload path reads `pixels` by reference.

Add an epoch-keyed `Arc<Vec<ImageModel>>` cache (same pattern as wire cache) so
`build_primitive()` performs only an O(1) Arc bump per frame during navigation:

```rust
image_cache: RefCell<Option<(u64, Arc<Vec<ImageModel>>)>>,
```

**Impact:** Eliminates MB-scale copies per frame in files that contain raster images.
With a 4K image (~32 MB raw), this alone can cut per-frame CPU time by tens of milliseconds.

**Difficulty:** Easy. Change one field type + add cache field + wire up in `build_primitive()`.

---

## Option H — Arc-wrap mesh models

**Status:** Done

Apply the same `Arc<Vec<MeshModel>>` epoch cache to mesh models:

```rust
mesh_cache: RefCell<Option<(u64, Arc<Vec<MeshModel>>)>>,
```

`MeshModel` contains `verts: Vec<[f32;3]>` (vertex positions) and `indices: Vec<u32>`
(triangle list). A complex 3D solid can have hundreds of thousands of triangles.

**Impact:** Eliminates per-frame mesh data copies in files that contain 3D solids (ACIS bodies,
extruded polylines). Navigation stays free regardless of model complexity.

**Difficulty:** Easy. Same pattern as Options F and G.

---

## Implementation order

F → G → H. Each is independent; all three can be done in a single pass since the pattern
is identical. F has the most complex rebuild logic (Insert explosion); G has the highest
raw byte impact; H is the simplest but matters for 3D-heavy files.
