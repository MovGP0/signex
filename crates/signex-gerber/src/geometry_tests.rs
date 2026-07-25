use std::io::Cursor;

use super::{ApertureShape, GerberPrimitive, PrimitivePolarity, load_gerber_reader};

const SIMPLE_LAYER: &str = r#"%FSLAX46Y46*%
%MOMM*%
%ADD10C,1.000*%
%ADD11R,2.000X1.000*%
D10*
X0000000Y0000000D02*
X0100000Y0000000D01*
X0100000Y0100000D03*
D11*
X0200000Y0100000D03*
M02*
"#;

#[test]
fn extracts_strokes_and_standard_flashes()
{
    let layer = load_gerber_reader("simple.gbr", Cursor::new(SIMPLE_LAYER))
        .expect("simple Gerber must parse");

    assert!(matches!(
        layer.geometry.primitives[0],
        GerberPrimitive::Stroke {
            width,
            polarity: PrimitivePolarity::Dark,
            ..
        } if (width - 1.0).abs() < 1e-9
    ));
    assert!(layer.geometry.primitives.iter().any(|primitive| matches!(
        primitive,
        GerberPrimitive::Flash {
            aperture: ApertureShape::Rectangle { width, height },
            ..
        } if (*width - 2.0).abs() < 1e-9 && (*height - 1.0).abs() < 1e-9
    )));
}
