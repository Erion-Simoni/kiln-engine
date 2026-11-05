# Editor design showcase

Open `kiln.toml` in the native editor and select Player in the hierarchy. This
small, real 2D scene follows the supplied HTML's blue player, green platform,
gold coins and gray obstacle. Play uses the ordinary runtime (arrows/WASD move
the player); it is a visual sample, without collision or game progression.

Click **3D** in the editor toolbar for the separate interactive perspective
preview. Drag to orbit, scroll to zoom, select meshes in the viewport/hierarchy,
and change their temporary properties in the inspector. These demonstration
objects are never written into this 2D project. Click **2D** to return.

The tiny original PNGs can be regenerated with `python3 art/generate.py` and are
covered by the repository license. No third-party art or HTML runtime is needed.
