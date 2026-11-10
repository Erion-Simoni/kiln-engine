# Authored sprite materials playground

Open kiln.toml in the editor or export it to Native/Browser using matching tools
and a material-capable runtime. The scene is shared by native/browser/editor pixel
acceptance. The editor Shader Editor can create shaders, parameters and materials;
assign a material to a sprite using the Inspector's sprite references.

- Top row: default blue, shader-default green, red Color override, dark-red Float
  override, then default blue. Default pipeline state must recover after custom draws.
- Middle: named Texture parameter defaults to red and overrides to blue.
- Bottom: overlapping default/custom/default sprites prove transparent painter
  ordering. A striped sprite flips horizontally; another rotates 90 degrees.
- S saves, N changes scenes, L restores the saved material references and pixels.

Shaders are fragment functions using VertexOutput and the standard sprite texture/
sampler. Numeric parameters become typed WGSL constants. A Texture parameter named
accent supplies accent and accent_sampler; images use ordinary project-relative
PNG resources/import options. See docs/SPRITE_MATERIALS_2D.md for the contract.
