# Soft mesh, timed crossfade and automatic blend tree

A ten-vertex strip has two bone influences at its intermediate vertices. Its
second bone bends, then straightens and extends; the strip deforms continuously.
One sprite crossfades red→green using an authored state transition. Another uses
an automatic speed-parameter blend tree, holding the red/green mix before green.
The blue beacon waits for completion. Final green bounds: [80,104,272,136].

Inspect skin → Soft Mesh Skinning to edit weights/bindings. Inspect fade/tree →
Animation States for the crossfade duration and threshold editor. The sample uses
ordinary project data and existing script parameters, with no generated assets
required at runtime. See [guide](../../docs/ADVANCED_ANIMATION_2D.md).

Pixel checks set KILN_PIXEL_REQUIRE_BLEND=1 to require observed mixed red/green
pixels in both sprite regions before accepting the final mesh and blue beacon.
