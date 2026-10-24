# Kiln Editor Roadmap — Unity/GameMaker-Class Editor

This document tracks the editor's evolution from a functional prototype to a
full-featured game development environment comparable to Unity or GameMaker.

## Current State (Implemented)

| Feature | Status |
|---------|--------|
| Native desktop app (egui + wgpu) | ✅ |
| Scene view drawn by engine sprite renderer | ✅ |
| Play-in-editor (PIE) with live slice runtime | ✅ |
| Asset browser with PNG previews | ✅ |
| Tilemap authoring (paint, solid flags, chunk scroll) | ✅ |
| Prefab create/instantiate | ✅ |
| Nested scenes with offsets | ✅ |
| Inspector with undo/redo (scene snapshots) | ✅ |
| Add/remove entities, Transform/Sprite/Role/Tilemap components | ✅ |
| Export/project pack via CLI | ✅ |
| Menu bar (File, Edit, etc.) | ✅ |

## Missing Features (Priority Order)

### Phase 1: Core Editor Infrastructure (This Session)

| # | Feature | Description |
|---|---------|-------------|
| 1.1 | **Scene Hierarchy Panel** | Tree view of all entities in the open scene, with selection and rename |
| 1.2 | **Enhanced Inspector** | Edit all component fields (Transform, Sprite, Role, Tilemap, AnimationPlayer) |
| 1.3 | **Node Graph Visual Editor** | Visual node editor for `kiln-graph` — nodes, pins, connections, compile to IR |
| 1.4 | **Animation Editor** | Sprite sheet clip authoring, frame editing, preview |
| 1.5 | **Build/Export UI** | In-editor `kiln-cli export` wrapper with platform selection |
| 1.6 | **Project Manager/Hub** | Create/open/recent projects, project settings editor |

### Phase 2: Advanced Tools

| # | Feature | Description |
|---|---------|-------------|
| 2.1 | **KilnScript Debugger** | Breakpoints, step-through, variable inspection |
| 2.2 | **Particle/VFX Editor** | Emitter configuration, particle preview |
| 2.3 | **Audio Editor** | Sound effect/music assignment, volume curves |
| 2.4 | **Physics Editor** | Collider shapes, rigid body settings, joint editor |
| 2.5 | **UI Layout Editor** | Visual UI canvas for kiln-ui widgets |
| 2.6 | **Profiler Integration** | GPU pass timings, frame budget overlay |

### Phase 3: Polish & Workflow

| # | Feature | Description |
|---|---------|-------------|
| 3.1 | **Preferences/Settings** | Editor theme, key bindings, external tool paths |
| 3.2 | **Recent Projects** | Auto-reopen last project, recent project list |
| 3.3 | **Scene Tabs** | Multiple open scenes, tab switching |
| 3.4 | **Prefab Editing UI** | Dedicated prefab editor with override view |
| 3.5 | **Version Control** | Git integration for project files |
| 3.6 | **Plugin System** | `kiln-plugin-api` editor panels, custom importers |

## Implementation Plan

Each phase is implemented as a set of editor panels/modules under `kiln-editor/src/`:

```
kiln-editor/src/
  app.rs              — Main app, dock layout, menu bar
  assets.rs           — Asset browser (exists)
  editing.rs          — Entity editing (exists)
  history.rs          — Undo/redo (exists)
  panels.rs           — Panel stubs → implement hierarchy, inspector, etc.
  prefab_editor.rs    — Prefab editing (exists)
  scene_view.rs       — Scene viewport (exists)
  tilemap_editor.rs   — Tilemap painting (exists)
  
  # New modules (this session):
  hierarchy.rs        — Scene hierarchy tree panel
  inspector.rs        — Enhanced component inspector
  graph_editor.rs     — Visual node graph editor
  anim_editor.rs      — Animation clip editor
  build_panel.rs      — Build/export UI
  project_hub.rs      — Project manager
```

## Data Flow

```
User edits in editor panel
    → EditorApp updates kiln_core::Project
    → Project saved to kiln.toml via Project::to_toml()
    → kiln-player loads kiln.toml at runtime
    → SliceSession runs KilnScript, reads/writes ECS world
    → SpriteRenderer draws to screen
```

## See Also

- `docs/ARCHITECTURE.md` — Crate map and data flow
- `docs/PROJECT_FORMAT.md` — kiln.toml schema reference
- `docs/KILNSCRIPT.md` — KilnScript language reference
- `kiln-editor/src/` — Editor source code