# Launchkey MK4 61 device drawing

Geometry revision 2 follows Novation's **61-key MK4** straight-top product photograph and numbered hardware overview. It is a functional vector drawing, with independently implemented shapes; no product photograph is bundled.

- [Official Launchkey 61 product page](https://novationmusic.com/products/launchkey-61)
- [Launchkey 61 hardware overview](https://userguides.novationmusic.com/hc/en-gb/articles/27613107564306-Launchkey-61-hardware-overview)
- [Dimensions: 895 × 264 × 93 mm](https://userguides.novationmusic.com/hc/en-gb/articles/27613113496338-Launchkey-61-s-specifications)

The canvas is 1000 × 295. Wheels and octave controls are above the keyboard on the left; nine faders are left of the central OLED and mode controls; eight encoders and sixteen pads are on the right. The far-right transport is Stop/Loop above Play/Record. The keybed contains 36 white and 25 black keys. Coordinates are measured proportions from the official photograph, not CAD measurements. The editor, preset thumbnails, selection targets, and reactive light origins share these coordinates.

`scripts/generate-layout.mjs` regenerates only the layout. `resources/layout-v1.json` is the frozen legacy geometry used to identify untouched v0.1.0 layouts. On load, those layouts receive the corrected geometry while preserving MIDI addresses, LED types and verification flags. Native storage keeps the original JSON backup. Manually moved controls or otherwise customized geometry are retained.

Physical placement does not establish MIDI or LED compatibility. The 42 existing LED IDs and addresses remain unchanged; non-lighting controls are drawn for orientation. Real-device acceptance testing remains required and the app does not mark them verified based on this drawing.
