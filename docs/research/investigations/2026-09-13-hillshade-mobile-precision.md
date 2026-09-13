# Hillshade Banding on Mobile GPUs

Status: Historical investigation

This investigation examined vertical bands and stepped shading edges in the
terrain hillshade on a Samsung Galaxy S23 with the nightly APK built from commit
`65ec44b`. The device used the Enroute France terrain file dated `13-Nov-2025`.
The map showed the waypoint `Coupe` at 44.0545°N 6.3296°E at about zoom 15.

## Conclusion

The nightly contained MapLibre GL JS 6.9.0, which includes the tile-seam fix
from MapLibre pull request 8302. That fix is present and is not the cause.

The bands come from a different defect. MapLibre's fragment shader prelude
declares `precision mediump float` on OpenGL ES. The `hillshade` fragment shader
does not override this precision. The `colorRelief` and `hillshadePrepare`
shaders do. Mobile GPUs implement `mediump` as 16-bit floats. The hillshade
shader reads the tile-local texture coordinate and maps it into the derivative
texture at this precision, so the coordinate is quantised to about 1/8 of a
DEM sample across the south-east three quarters of each tile. Desktop GPUs and
SwiftShader implement `mediump` as 32-bit floats, so the defect does not appear
there.

The Enroute terrain files stop at zoom 10. At zoom 15 one DEM sample covers 32
CSS pixels, so the quantisation steps become visible as bands and stair steps.
The pre-8302 shader had the same quantisation. The fix did not introduce the
problem and did not make it materially worse.

## Evidence

1. The MapLibre changelog at tags `v6.8.0` and `v6.9.0` lists pull request 8302
   under 6.8.0. The installed 6.9.0 bundle contains the 2-pixel DEM padding and
   the new border fill from that change.
2. The France terrain file has zoom levels 7 to 10 with 256-pixel lossless WebP
   tiles. All zoom-10 tiles around `Coupe` exist. `Coupe` lies 130 m east of
   the boundary between zoom-10 columns 529 and 530.
3. A local MapLibre 6.9.0 render of the same view, style, and tiles in headless
   Chromium with SwiftShader showed smooth shading without bands.
4. The same render with the hillshade fragment shader patched to round the
   varying and each intermediate coordinate to 16-bit floats reproduced the
   phone screenshot: vertical bands and a stepped diagonal light-to-dark edge
   south-west of `Coupe`, in the same positions.
5. Rounding only the varying, which models the pre-8302 shader, produced the
   same bands.
6. A 16-bit float in the range 0.5 to 1 has a spacing of 1/2048. Scaled to a
   258-pixel texture, that is 0.126 samples. The band region lies in the eastern
   part of column 529, where the tile-local x coordinate exceeds 0.75. The
   north-west quarter of a tile has 16 times finer spacing and shows no bands.

## Eliminated causes

The evidence ruled out:

- a nightly build without the MapLibre seam fix
- a terrain file with missing, lossy, or misaligned tiles near `Coupe`
- the colour-relief layer, which declares `highp` and samples the DEM directly
- the `hillshadePrepare` pass, which declares `highp` and uses `texelFetch`
- map rotation or pitch, which the flight view does not apply

## Mitigation assessment

The fix belongs in MapLibre. The `hillshade` fragment shader needs
`precision highp float` under `GL_ES`, as `colorRelief` already has. Moving the
coordinate mapping into the vertex shader is not sufficient, because the
fragment shader still receives the varying at `mediump`.

Updraft cannot change the shader without patching MapLibre. Overzoom of the
zoom-10 Enroute terrain remains the normal case for the flight view, so the
bands will stay visible until MapLibre ships the precision change.

## Limits

The phone artifacts were reproduced by emulation, not on the device. The
emulation rounded values with `packHalf2x16`, which models IEEE half floats.
A GPU driver can use a different internal precision for `mediump`, so the exact
band width on a device can differ from the emulation.

The investigation used MapLibre GL JS 6.9.0 at commit `65ec44b`. Recheck the
shader precision before applying these findings to a different MapLibre
version.
