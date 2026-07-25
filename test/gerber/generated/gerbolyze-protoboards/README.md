# Gerbolyze protoboard fixtures

These fabrication archives are test fixtures derived from the board definitions
in Gerbolyze's
[`generate_protoboards.py`](https://github.com/jaseg/gerbolyze/blob/main/generate_protoboards.py).
They are intended to exercise Gerber and Excellon import without depending on
KiCad source code.

## Provenance

- Generated on: 2026-07-25
- Python: 3.12.10
- Gerbolyze source revision:
  `b07f461be3d29581c42ec9507f77e562126de9d6`
- Gerbonara: 1.6.3
- Local generator:
  `D:\GitHub\gerbolyze\generate_protoboards.py`
- Local generator SHA-256:
  `4A6C25AE4D15F69C6FDF2713F8C79DC01F20DE83CC91E670D5411585C013146F`

The checked-out generator imports `gerbolyze.protoboard`, which is no longer
present at the current Gerbolyze revision. The compatible module was obtained
from Gerbolyze revision
`b1324e9a537266ca83bb18ef48fb272134134e92` and had SHA-256
`55366F0A903FEDAFF6E1455AD90E5BDA66115928903B87E5CA0CE50B10AD89C5`.
The local generator and compatibility module successfully produced the
intermediate layered SVG boards.

Gerbolyze's Windows WASI conversion dependencies could not convert those SVGs:
one current wheel contained an incompatible WebAssembly component and another
omitted its expected `.wasm` payload. The final fabrication archives were
therefore written directly through Gerbonara's CAD API, using the same numeric
board definitions from the linked generator:

- `THTPads()`: 2.54 mm pitch, 2.0 mm pad, 1.0 mm drill
- `SMDPads(0.65, 2.0)`: 0.65 mm pitch and 2.0 mm row spacing
- `SMDPads(1.27, 2.54)`: 1.27 mm pitch and 2.54 mm row spacing
- 20 mm x 30 mm boards with 2.2 mm non-plated mounting holes

This records a reproducible fallback rather than claiming that the archives
were emitted by Gerbolyze's `convert` command.

## Archives

| Archive | Coverage | SHA-256 |
| --- | --- | --- |
| `tht_pitch100mil_20x30.zip` | Through-hole pads and plated drills on a 100 mil grid | `39A2943FCF5C2BA6E87707C69977178237DEA02BDBA1D88260827F98CA0DCA40` |
| `smd_sop650_double_side_20x30.zip` | Double-sided 0.65 mm SMD pad arrays | `ABDF6B39229FFA93657598440CC2D35CC504BF0507EB864AB3EAC829648692D0` |
| `mixed_tht_smd_double_side_20x30.zip` | Through-hole pads plus double-sided 1.27 mm SMD arrays | `9ADF851D1B370519F5EF4307850BF8C1E4C568D229FDF8B5A5A4930783AE64B5` |

Each ZIP contains:

- `proto-F_Cu.gbr` and `proto-B_Cu.gbr`
- `proto-F_Mask.gbr` and `proto-B_Mask.gbr`
- `proto-F_Paste.gbr` and `proto-B_Paste.gbr`
- `proto-F_SilkS.gbr` and `proto-B_SilkS.gbr`
- `proto-Edge_Cuts.gbr`
- `proto-PTH.drl`
- `proto-NPTH.drl`

All member paths were checked for absolute paths and parent traversal. Each
archive was reopened by `gerbonara.LayerStack.open`, which recognized the
fabrication layers and reported bounds of approximately 20.05 mm x 30.05 mm.
The parser warns that the generated edge-cut arcs do not explicitly enable
multi-quadrant interpolation with `G75`. The files are retained this way as a
useful robustness fixture for older or stricter Gerber interpreters.

## Licensing

Gerbolyze's source code is licensed under AGPL-3.0-or-later. Its generated
protoboard index separately states that the downloadable protoboard outputs are
provided under the Unlicense. These archives contain generated board output,
not copied Gerbolyze source code.
