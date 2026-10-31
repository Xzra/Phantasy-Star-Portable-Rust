# Presentation pass

The game now embeds unmodified Liberation Sans Regular for readable interface
text. Its SIL Open Font License and copyright notices are included in
`assets/fonts/OFL.txt` and `THIRD_PARTY_NOTICES.md`; the game needs no installed
font or runtime download. The model viewer retains its existing presentation.

The original lobby uses darker segmented deck plates, recessed guidance strips
and structural framing outside its walkable area. The forest uses a continuous
vertex-colored dirt clearing, a denser ring of faceted canopies and low grass.
New distant tree geometry sits outside arena collision bounds. The clearing
stays flat where the player walks; decorative foliage stays below attack
telegraphs. No terrain or navigation behavior is inferred from the scenery.

Static environment meshes are built once and split into batches of at most
4,500 vertices. Face shading and interpolated ground colors supply depth without
textures. These are original procedural assets: this pass does not import
Classic PSO2 stages or implement a physically based lighting pipeline.

HUD and equipment panels share a dark framed treatment, restrained accent
lines, shadows and the new font. Mira's supply counter uses a warm gold accent
to distinguish commerce from cyan mission/inventory controls. Menu cursors remain
visible above panels. The shopkeeper has an original uniform, idle head motion,
a counter display, collision and a projected nameplate.

Local application framebuffer captures cover lobby, inventory, forest, the
shopkeeper and shop menu. Full graphical trials verify the presentation remains
compatible with ordinary combat and extraction.
