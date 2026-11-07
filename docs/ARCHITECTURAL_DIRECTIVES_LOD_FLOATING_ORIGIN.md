# SDK Architectural Directives: Hierarchical Chunked LOD & Floating-Origin Platform Standard

This document outlines the authoritative planetary simulation and terrain streaming standards for the **Kuroshio Gyre**, **Nautilus**, and **Lemuria** platforms, aligning the SDK and client engines with the industry gold standard (Flight Simulator, Cesium, Star Citizen).

---

## 1. Standardize on Hierarchical Chunked LOD (CDLOD) with Quadtree Streaming
> *"The SDK should natively provide a quadtree chunk manager for `crates/nautilus-world`. Ground meshes shouldn't be loaded as monolithic meshes. The engine should maintain a reusable pool of terrain chunk buffers that slide and stream based on the player's WGS84 tile coordinate."*

### Implementation Directives:
- **Zero Monolithic Meshes:** Never instantiate a single static terrain plane (e.g. $280\text{m} \times 280\text{m}$) or clamp movements to localized bounds (`Math.max(-140, Math.min(140, ...))`).
- **Dynamic Reusable Chunk Pool:** Maintain an active sliding grid of terrain chunk buffers ($3 \times 3$ or $5 \times 5$ quad chunks) centered dynamically on the observer's camera-relative coordinate. Chunks falling behind player movement vectors are recycled in $O(1)$ and shifted forward.
- **Continuous Distance-dependent LOD (CDLOD):**
  - **Tier 1 (High Detail):** $0\text{m} - 150\text{m}$ (dense micro-relief, 5ft tactical grid, Rapier3D physics colliders).
  - **Tier 2 (Mid Detail):** $150\text{m} - 1,500\text{m}$ (coarse elevation sampling, continuous geomorphing across LOD steps to eliminate pop-in).
  - **Tier 3 (Distant):** $>1,500\text{m}$ (subdivided quadtree boundaries).
- **Continuous Elevation Ingestion:** Query `CloudOptimizedDemSampler` or procedural harmonic functions (`getTerrainHeight(lat, lon)`) seamlessly without edge boundaries.

---

## 2. Implement Floating-Origin Camera Repositioning
> *"To allow players to walk across real-world countries (hundreds of kilometers) without hitting 32-bit GPU floating-point precision jitter, the engine must implement a floating origin: whenever the player moves beyond 500m from the local coordinate anchor, the camera world origin resets to (0,0,0) and advances the geodetic reference anchor."*

### Implementation Directives:
- **Dual-Coordinate Precision:** Store all world objects, landmarks, player avatars, and chunk boundaries in authoritative 64-bit (`f64`) Earth-Centered, Earth-Fixed (ECEF) coordinates.
- **Vertex-Shader Camera-Relative Injection:** Subtract the active 64-bit floating origin from world positions before submitting vertex buffers to the GPU:
  $$\vec{v}_{\text{rel}} = (\vec{X}_{\text{ECEF}} - \vec{C}_{\text{ECEF}})_{f32}$$
- **Automatic Re-Origin Trigger:** When Euclidean distance $\lVert \vec{C}_{\text{player}} - \vec{O}_{\text{anchor}} \rVert > 500\text{m}$, reset the local coordinate origin to $\vec{C}_{\text{player}}$ and advance the WGS84 geodetic reference anchor. This eliminates 32-bit vertex wobbling and depth-fighting over journeys spanning thousands of kilometers.

---

## 3. Separate Near-Field Geometry from Far-Field Horizon Impostors
> *"Provide native billboard/impostor shaders for distant cities, mountain ranges, and orbital beacons that render in a separate far-plane depth buffer (1.5km to 50km), allowing infinite horizons with minimal draw calls."*

### Implementation Directives:
- **Depth-Buffer Separation:** Render near-field tactical terrain and avatars in the primary depth buffer ($0.1\text{m} - 2,000\text{m}$). Render far-field landmarks, distant mountain silhouettes (e.g. Mount Erciyes), and city skylines in a secondary far-plane depth buffer ($1.5\text{km} - 50\text{km}$).
- **Cross-Fade Transitions:**
  - Between $750\text{m}$ and $1,500\text{m}$, cross-fade the far-field silhouette alpha into full-fidelity CDLOD chunks.
  - Between $40\text{km}$ and $50\text{km}$, apply atmospheric Rayleigh/Mie scattering fog fade-in.
- **Minimal Draw Call Budget:** Consolidate hundreds of distant landmarks into instanced billboard buffers or simplified low-poly silhouettes.

---

## 4. Scaled Travel Modes & Physical Kinetics
- **Walking Mode:** $1.4\text{ m/s}$ ($5.0\text{ km/h}$) — Tactical close-quarters combat, storefront exploration, dungeon crawling.
- **Mounted / Steed Mode:** $8.3 - 11.1\text{ m/s}$ ($30 - 40\text{ km/h}$) — Rapid overland travel between neighboring sectors (15–30 minutes between regional outposts).
- **Tactical Vehicle Mode:** $22.2 - 33.3\text{ m/s}$ ($80 - 120\text{ km/h}$) — Highway couriers, inter-city transport.
- **Atmospheric Flight Mode:** $150\text{ m/s}$ ($540\text{ km/h}$) — Stratospheric transit and orbital insertion.
