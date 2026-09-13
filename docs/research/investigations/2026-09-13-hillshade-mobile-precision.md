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

## Origin of the precision qualifier

The `mediump` default is not a hillshade decision. Mapbox GL JS declared
`precision mediump float` in the shared fragment prelude before 2017, when the
shaders targeted WebGL 1. GLSL ES 1.00 makes `highp` optional in fragment
shaders, so `mediump` was the portable default. MapLibre kept the prelude
unchanged in the GLSL ES 3.00 migration of May 2023 (pull request 2599), which
made `highp` mandatory in fragment shaders and removed the portability reason.

The hillshade shaders arrived with Mapbox pull request 5286 in December 2017.
Its author added `precision highp float` to `hillshade_prepare` to fix a
rendering failure on Android, because elevation decoding needs more than 16-bit
mantissa. The draw shader `hillshade` received no qualifier and inherited the
prelude default. The pull request contains no discussion of texture coordinate
precision or overzoom precision.

MapLibre later added `highp` to specific shaders when precision defects
appeared: `fill_pattern` and `line_pattern` in pull request 416 (2021, pattern
misalignment when overzoomed on OpenGL ES devices), `color_relief` in pull
request 5925 (2025, elevation decoding), and `atmosphere` in pull request 6939.
Pull request 416 limited the change to two shaders to "limit the impact to
memory consumption and performance", without measurements. Issue 579 raised the
WebGL 1 compatibility concern for `highp` and closed as stale. MapLibre Native
uses the same `mediump` prelude and the same unqualified `hillshade` fragment
shader.

The evidence shows an inherited default that nobody revisited for this shader,
not a deliberate performance choice.

## Cost of `highp`

Vendor guidance gives the upper bound. Qualcomm states that Adreno executes
16-bit arithmetic at twice the rate of 32-bit and with half the register
footprint. Arm states that Mali interpolates 16-bit varyings at twice the rate
of 32-bit varyings and stores twice as many 16-bit values per register.

The `hillshade` draw pass runs once per screen pixel per rendered frame. Its
fragment shader performs one texture fetch, one `textureSize` query, about ten
arithmetic operations for the coordinate and the latitude scale, and for the
Igor method about twenty arithmetic operations plus `cos`, `atan`, `atan2`,
`sqrt`, and `mod`. Transcendental functions run in dedicated units whose rate
does not double at 16 bits. A whole-shader `highp` therefore at most doubles
the arithmetic part of a shader that is dominated by the texture fetch and the
transcendental functions.

On the Galaxy S23 the pass covers about 2.5 million fragments per frame. At an
arithmetic throughput above one teraflop, the extra 32-bit work stays below
0.1 ms per frame, which is under 1% of a 16.7 ms frame. MapLibre renders only
when the map changes, so the cost applies to moving frames only.

A narrower change avoids even that cost. Declaring `in highp vec2 v_pos` and
computing `texturePos` as `highp` fixes the texture coordinate and leaves the
shading arithmetic at `mediump`. MapLibre already uses this pattern for the
`v_uv` texture coordinate in the `line_gradient` fragment shaders. The
interpolation of one 32-bit varying and five 32-bit operations per fragment is
not measurable.

## Mitigation assessment

The fix belongs in MapLibre. The `hillshade` fragment shader needs the texture
coordinate at `highp`, either through `precision highp float` under `GL_ES` as
`colorRelief` has, or through a `highp` qualifier on `v_pos` and `texturePos`.
Moving the coordinate mapping into the vertex shader is not sufficient, because
the fragment shader still receives the varying at `mediump`.

Updraft applies the narrow change as a pnpm patch to `maplibre-gl` 6.9.0 in
`patches/maplibre-gl@6.9.0.patch`. A nightly APK with this patch showed smooth
hillshade at the same view on the Galaxy S23. The patch stays until MapLibre
ships the precision change.

## Limits

The phone artifacts were reproduced by emulation. The emulation rounded values
with `packHalf2x16`, which models IEEE half floats. A GPU driver can use a
different internal precision for `mediump`, so the exact band width on a device
can differ from the emulation. The device check confirmed the fix on one Adreno
GPU only.

The investigation used MapLibre GL JS 6.9.0 at commit `65ec44b`. Recheck the
shader precision before applying these findings to a different MapLibre
version.
