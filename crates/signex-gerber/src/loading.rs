use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

use lib_gerber_edit::layer::{LayerData, LayerType};

use crate::GerberGeometry;

/// A successfully parsed fabrication layer and its render-ready geometry.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadedLayer
{
    pub source_path: Option<PathBuf>,
    pub name: String,
    pub layer_type: LayerType,
    pub data: LayerData,
    pub geometry: GerberGeometry,
}

/// An actionable per-file loading failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GerberLoadFailure
{
    pub path: PathBuf,
    pub message: String,
}

impl std::fmt::Display for GerberLoadFailure
{
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
    {
        write!(formatter, "{}: {}", self.path.display(), self.message)
    }
}

/// Partial-success result for a multi-file load.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GerberLoadBatch
{
    pub layers: Vec<LoadedLayer>,
    pub failures: Vec<GerberLoadFailure>,
}

/// Loads one RS-274X file from disk.
pub fn load_gerber_file(path: impl AsRef<Path>) -> Result<LoadedLayer, GerberLoadFailure>
{
    let path = path.as_ref();
    let file = File::open(path).map_err(|error| GerberLoadFailure {
        path: path.to_path_buf(),
        message: format!("could not open file: {error}"),
    })?;
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    let mut layer = load_gerber_reader(name, file).map_err(|mut failure| {
        failure.path = path.to_path_buf();
        failure
    })?;
    layer.source_path = Some(path.to_path_buf());
    Ok(layer)
}

/// Loads several RS-274X files, preserving successful layers when another file fails.
pub fn load_gerber_files<I, P>(paths: I) -> GerberLoadBatch
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
{
    let mut batch = GerberLoadBatch::default();
    for path in paths
    {
        match load_gerber_file(path.as_ref())
        {
            Ok(layer) => batch.layers.push(layer),
            Err(failure) => batch.failures.push(failure),
        }
    }
    batch
}

/// Loads one RS-274X layer from an arbitrary reader.
///
/// This is the common parser seam used by disk loading and archive tests.
pub fn load_gerber_reader<R>(
    name: impl Into<String>,
    reader: R,
) -> Result<LoadedLayer, GerberLoadFailure>
where
    R: Read,
{
    let name = name.into();
    let path = PathBuf::from(&name);
    let extension = Path::new(&name)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    let requested_type = match extension.to_ascii_lowercase().as_str()
    {
        "ger" | "pho" => LayerType::UndefinedGerber,
        _ => LayerType::try_from(extension).map_err(|_| GerberLoadFailure {
            path: path.clone(),
            message: format!("unsupported Gerber file extension '.{extension}'"),
        })?,
    };

    if requested_type == LayerType::Drill
    {
        return Err(GerberLoadFailure {
            path,
            message: "the file is an Excellon drill layer, not an RS-274X Gerber layer".into(),
        });
    }

    let (layer_type, data) = LayerData::parse(requested_type, BufReader::new(reader)).map_err(
        |error| GerberLoadFailure {
            path: path.clone(),
            message: error.to_string(),
        },
    )?;
    let LayerData::Gerber(data) = data else
    {
        return Err(GerberLoadFailure {
            path,
            message: "the file was detected as Excellon drill data".into(),
        });
    };
    let geometry = GerberGeometry::from_layer(&data);

    Ok(LoadedLayer {
        source_path: None,
        name,
        layer_type,
        data: LayerData::Gerber(data),
        geometry,
    })
}

/// Loads one Excellon drill file from disk.
pub fn load_excellon_file(path: impl AsRef<Path>) -> Result<LoadedLayer, GerberLoadFailure>
{
    let path = path.as_ref();
    let file = File::open(path).map_err(|error| GerberLoadFailure {
        path: path.to_path_buf(),
        message: format!("could not open file: {error}"),
    })?;
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    let mut layer = load_excellon_reader(name, file).map_err(|mut failure| {
        failure.path = path.to_path_buf();
        failure
    })?;
    layer.source_path = Some(path.to_path_buf());
    Ok(layer)
}

/// Loads several Excellon files, preserving successful layers when another file fails.
pub fn load_excellon_files<I, P>(paths: I) -> GerberLoadBatch
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
{
    let mut batch = GerberLoadBatch::default();
    for path in paths
    {
        match load_excellon_file(path.as_ref())
        {
            Ok(layer) => batch.layers.push(layer),
            Err(failure) => batch.failures.push(failure),
        }
    }
    batch
}

/// Loads one Excellon drill layer from an arbitrary reader.
pub fn load_excellon_reader<R>(
    name: impl Into<String>,
    reader: R,
) -> Result<LoadedLayer, GerberLoadFailure>
where
    R: Read,
{
    let name = name.into();
    let path = PathBuf::from(&name);
    let extension = Path::new(&name)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    let requested_type = LayerType::try_from(extension).map_err(|_| GerberLoadFailure {
        path: path.clone(),
        message: format!("unsupported Excellon file extension '.{extension}'"),
    })?;
    if requested_type != LayerType::Drill
    {
        return Err(GerberLoadFailure {
            path,
            message: "the file extension does not identify an Excellon drill layer".into(),
        });
    }

    let (_, data) = LayerData::parse(LayerType::Drill, BufReader::new(reader)).map_err(
        |error| GerberLoadFailure {
            path: path.clone(),
            message: error.to_string(),
        },
    )?;
    let LayerData::Excellon(data) = data else
    {
        return Err(GerberLoadFailure {
            path,
            message: "the file was detected as RS-274X Gerber data".into(),
        });
    };
    let geometry = GerberGeometry::from_excellon(&data);

    Ok(LoadedLayer {
        source_path: None,
        name,
        layer_type: LayerType::Drill,
        data: LayerData::Excellon(data),
        geometry,
    })
}
