use lib_gerber_edit::gerber_types::{
    Aperture, Command, CommentContent, ExtendedCode, FunctionCode, GCode, StandardComment, Unit,
};
use lib_gerber_edit::layer::LayerData;

use crate::{Bounds, LoadedLayer};

/// Display-ready summary of the parsed data available for one fabrication layer.
#[derive(Debug, Clone, PartialEq)]
pub struct LayerMetadata
{
    pub file_name: String,
    pub source: String,
    pub format: &'static str,
    pub layer_role: String,
    pub units: String,
    pub coordinate_format: Option<String>,
    pub bounds: Option<Bounds>,
    pub primitive_count: usize,
    pub definition_label: &'static str,
    pub definitions: Vec<String>,
    pub attributes: Vec<String>,
    pub warnings: Vec<String>,
}

impl LoadedLayer
{
    pub fn metadata(&self) -> LayerMetadata
    {
        let source = self
            .source_path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "In-memory source".to_owned());
        let layer_role = self.layer_type.to_string().trim().to_owned();

        match &self.data
        {
            LayerData::Gerber(layer) => {
                let format = &layer.coordinate_format;
                let mut definitions = layer
                    .apertures
                    .iter()
                    .map(|(code, aperture)| {
                        format!("D{code}: {}", describe_aperture(aperture))
                    })
                    .collect::<Vec<_>>();
                definitions.sort();
                let attributes = layer
                    .header
                    .iter()
                    .chain(&layer.commands)
                    .filter_map(attribute_description)
                    .collect();

                LayerMetadata {
                    file_name: self.name.clone(),
                    source,
                    format: "Gerber RS-274X",
                    layer_role,
                    units: "Millimetres (normalized)".to_owned(),
                    coordinate_format: Some(format!(
                        "{}.{} digits, {:?}, {:?}",
                        format.integer,
                        format.decimal,
                        format.zero_omission,
                        format.coordinate_mode,
                    )),
                    bounds: self.geometry.bounds,
                    primitive_count: self.geometry.primitives.len(),
                    definition_label: "Apertures",
                    definitions,
                    attributes,
                    warnings: self.geometry.warnings.clone(),
                }
            }
            LayerData::Excellon(layer) => {
                let mut definitions = layer
                    .tools
                    .iter()
                    .map(|(tool, diameter)| {
                        format!(
                            "T{tool}: {diameter:.4} {}",
                            unit_suffix(layer.unit.unit),
                        )
                    })
                    .collect::<Vec<_>>();
                definitions.sort();

                LayerMetadata {
                    file_name: self.name.clone(),
                    source,
                    format: "Excellon drill",
                    layer_role,
                    units: unit_name(layer.unit.unit).to_owned(),
                    coordinate_format: Some(layer.unit.to_string()),
                    bounds: self.geometry.bounds,
                    primitive_count: self.geometry.primitives.len(),
                    definition_label: "Tools",
                    definitions,
                    attributes: Vec::new(),
                    warnings: self.geometry.warnings.clone(),
                }
            }
            LayerData::Info(_) => LayerMetadata {
                file_name: self.name.clone(),
                source,
                format: "Text information",
                layer_role,
                units: "Not applicable".to_owned(),
                coordinate_format: None,
                bounds: None,
                primitive_count: 0,
                definition_label: "Definitions",
                definitions: Vec::new(),
                attributes: Vec::new(),
                warnings: Vec::new(),
            },
        }
    }
}

fn describe_aperture(aperture: &Aperture) -> String
{
    match aperture
    {
        Aperture::Circle(circle) => format!("circle, ⌀{:.4} mm", circle.diameter),
        Aperture::Rectangle(rectangle) => {
            format!("rectangle, {:.4} ⨯ {:.4} mm", rectangle.x, rectangle.y)
        }
        Aperture::Obround(rectangle) => {
            format!("obround, {:.4} ⨯ {:.4} mm", rectangle.x, rectangle.y)
        }
        Aperture::Polygon(polygon) => format!(
            "polygon, ⌀{:.4} mm, {} vertices, {:.2}°",
            polygon.diameter,
            polygon.vertices,
            polygon.rotation.unwrap_or_default(),
        ),
        Aperture::Macro(name, _) => format!("macro, {name}"),
    }
}

fn attribute_description(command: &Command) -> Option<String>
{
    match command
    {
        Command::ExtendedCode(ExtendedCode::FileAttribute(attribute)) => {
            Some(format!("File: {attribute:?}"))
        }
        Command::ExtendedCode(ExtendedCode::ApertureAttribute(attribute)) => {
            Some(format!("Aperture: {attribute:?}"))
        }
        Command::ExtendedCode(ExtendedCode::ObjectAttribute(attribute)) => {
            Some(format!("Object: {attribute:?}"))
        }
        Command::FunctionCode(FunctionCode::GCode(GCode::Comment(
            CommentContent::Standard(StandardComment::FileAttribute(attribute)),
        ))) => Some(format!("File: {attribute:?}")),
        Command::FunctionCode(FunctionCode::GCode(GCode::Comment(
            CommentContent::Standard(StandardComment::ApertureAttribute(attribute)),
        ))) => Some(format!("Aperture: {attribute:?}")),
        Command::FunctionCode(FunctionCode::GCode(GCode::Comment(
            CommentContent::Standard(StandardComment::ObjectAttribute(attribute)),
        ))) => Some(format!("Object: {attribute:?}")),
        _ => None,
    }
}

fn unit_name(unit: Unit) -> &'static str
{
    match unit
    {
        Unit::Inches => "Inches",
        Unit::Millimeters => "Millimetres",
    }
}

fn unit_suffix(unit: Unit) -> &'static str
{
    match unit
    {
        Unit::Inches => "in",
        Unit::Millimeters => "mm",
    }
}

#[cfg(test)]
mod tests
{
    use std::io::Cursor;

    #[test]
    fn gerber_metadata_reports_normalized_format_apertures_and_attributes()
    {
        let layer = crate::load_gerber_reader(
            "copper.gtl",
            Cursor::new(
                b"%FSLAX46Y46*%\n%MOIN*%\n%TF.GenerationSoftware,Signex,Fixture,1.0*%\n%ADD10C,0.010*%\nD10*\nX0Y0D03*\nM02*\n",
            ),
        )
        .expect("test Gerber must parse");

        let metadata = layer.metadata();

        assert_eq!(metadata.file_name, "copper.gtl");
        assert_eq!(metadata.format, "Gerber RS-274X");
        assert_eq!(metadata.units, "Millimetres (normalized)");
        assert_eq!(
            metadata.coordinate_format.as_deref(),
            Some("4.6 digits, Leading, Absolute"),
        );
        assert_eq!(metadata.definition_label, "Apertures");
        assert_eq!(metadata.definitions, vec!["D10: circle, ⌀0.2540 mm"]);
        assert_eq!(metadata.primitive_count, 1);
        assert!(
            metadata
                .attributes
                .iter()
                .any(|attribute| attribute.contains("GenerationSoftware")),
        );
    }

    #[test]
    fn excellon_metadata_reports_declared_unit_and_sorted_tools()
    {
        let layer = crate::load_excellon_reader(
            "holes.drl",
            Cursor::new(
                b"M48\nMETRIC\nT02C1.2\nT01C0.8\n%\nG05\nT01\nX1.0Y1.0\nM30\n",
            ),
        )
        .expect("test Excellon must parse");

        let metadata = layer.metadata();

        assert_eq!(metadata.format, "Excellon drill");
        assert_eq!(metadata.units, "Millimetres");
        assert_eq!(metadata.definition_label, "Tools");
        assert_eq!(
            metadata.definitions,
            vec!["T1: 0.8000 mm", "T2: 1.2000 mm"],
        );
        assert_eq!(metadata.primitive_count, 1);
        assert!(metadata.attributes.is_empty());
    }
}
