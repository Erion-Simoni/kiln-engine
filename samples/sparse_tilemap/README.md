# Sparse tilemap

One visible green tile and another at tile X=2,000,000,000 share two chunks.
The visible tile renders at bounds [128,112,192,176]. Huge view/collision queries
are covered by core/runtime tests; this fixture checks normal export pixels.
