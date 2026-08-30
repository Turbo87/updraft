# Third-party licenses

## Positron map style

`frontend/src/lib/map/style/positron.json` contains a snapshot of the Positron
style served by [OpenFreeMap](https://openfreemap.org). The snapshot was
retrieved on 2026-08-29.

The style is a fork of
[openmaptiles/positron-gl-style](https://github.com/openmaptiles/positron-gl-style).
It is derived from "CartoDB Basemaps", which Stamen and Paul Norman designed
for CartoDB Inc. under the
[CC BY 3.0 license](https://creativecommons.org/licenses/by/3.0/).

Copyright (c) 2024, MapTiler.com & OpenMapTiles contributors.
Copyright (c) 2015, CartoDB Inc.
All rights reserved.

The style code uses the BSD 3-Clause License:

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:

- Redistributions of source code must retain the above copyright notice,
  this list of conditions and the following disclaimer.
- Redistributions in binary form must reproduce the above copyright notice,
  this list of conditions and the following disclaimer in the documentation
  and/or other materials provided with the distribution.
- Neither the name of the copyright holder nor the names of its contributors
  may be used to endorse or promote products derived from this software
  without specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE
LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
POSSIBILITY OF SUCH DAMAGE.

The visual design uses the
[CC BY 4.0 license](https://creativecommons.org/licenses/by/4.0/).

## Basemap fonts

`frontend/static/basemap/fonts` contains MapLibre glyph files for Barlow Semi
Condensed Medium, Medium Italic, and Bold. The stacks use Noto Sans with the
same weight and style as a fallback. This adds Greek and Cyrillic coverage.

The Barlow files come from
[Google Fonts revision `ade3d153`](https://github.com/google/fonts/tree/ade3d1533e06b2b1462ffcde8e08b129627ca360/ofl/barlowsemicondensed).
The Noto Sans files come from the
[Noto Sans 2.015 release](https://github.com/notofonts/latin-greek-cyrillic/releases/tag/NotoSans-v2.015).

[Barlow](https://tribby.com/fonts/barlow/) is copyright 2017 The Barlow Project
Authors. [Noto Sans](https://fonts.google.com/noto) is copyright Google LLC.
Both fonts use the
[SIL Open Font License, Version 1.1](https://openfontlicense.org/).

`frontend/static/basemap/sprites` contains the OpenFreeMap sprite sheets that
the Positron style uses. The sprite sheets were retrieved on 2026-08-29 from
`https://tiles.openfreemap.org/sprites/ofm_f384/ofm` at 1x and 2x resolutions.
The sheets use the [Maki](https://github.com/mapbox/maki) POI icon set under the
[CC0 1.0 license](https://creativecommons.org/publicdomain/zero/1.0/). The
right-arrow icon is in the public domain.

## Tailwind CSS

`frontend/src/styles/colors.ts` contains the default color palette from
Tailwind CSS v4.3.0. The generated `frontend/src/styles/colors.generated.css`
and `frontend/src/lib/map/colors.generated.ts` files contain derived forms of
the same palette:

https://github.com/tailwindlabs/tailwindcss/blob/v4.3.0/packages/tailwindcss/theme.css

MIT License

Copyright (c) Tailwind Labs, Inc.

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
