use std::fmt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const MILLIMETRES_PER_MIL: f64 = 0.0254;
const SETTINGS_FILE_NAME: &str = "gerber_viewer.toml";
// Use the en-US decimal point only when the operating-system locale cannot be read.
const DEFAULT_DECIMAL_SEPARATOR: &str = ".";
pub(super) const DEFAULT_GRID_INDEX: usize = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum GridUnit
{
    Mil,
    Mm,
}

impl GridUnit
{
    pub const ALL: [Self; 2] = [Self::Mil, Self::Mm];
}

impl fmt::Display for GridUnit
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        formatter.write_str(match self
        {
            Self::Mil => "mils",
            Self::Mm => "mm",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(super) struct GridSizePreset
{
    pub name: Option<String>,
    pub x: f64,
    pub y: f64,
    pub unit: GridUnit,
}

impl GridSizePreset
{
    pub fn x_millimetres(&self) -> f64
    {
        match self.unit
        {
            GridUnit::Mil => self.x * MILLIMETRES_PER_MIL,
            GridUnit::Mm => self.x,
        }
    }

    pub fn y_millimetres(&self) -> f64
    {
        match self.unit
        {
            GridUnit::Mil => self.y * MILLIMETRES_PER_MIL,
            GridUnit::Mm => self.y,
        }
    }

    fn x_mils(&self) -> f64
    {
        match self.unit
        {
            GridUnit::Mil => self.x,
            GridUnit::Mm => self.x / MILLIMETRES_PER_MIL,
        }
    }

    fn y_mils(&self) -> f64
    {
        match self.unit
        {
            GridUnit::Mil => self.y,
            GridUnit::Mm => self.y / MILLIMETRES_PER_MIL,
        }
    }

    pub fn display_label(&self, decimal_separator: &str) -> String
    {
        let mils = format_dimensions(
            self.x_mils(),
            self.y_mils(),
            2,
            "mils",
            decimal_separator,
        );
        let millimetres = format_dimensions(
            self.x_millimetres(),
            self.y_millimetres(),
            4,
            "mm",
            decimal_separator,
        );
        let dimensions = match self.unit
        {
            GridUnit::Mil => format!("{mils} ({millimetres})"),
            GridUnit::Mm => format!("{millimetres} ({mils})"),
        };

        match self.name.as_deref()
        {
            Some(name) if !name.trim().is_empty() => format!("{name}: {dimensions}"),
            _ => dimensions,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct GerberViewerSettings
{
    grid_sizes: Vec<GridSizePreset>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct GridSizeChoice
{
    pub index: usize,
    label: String,
}

impl fmt::Display for GridSizeChoice
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        formatter.write_str(&self.label)
    }
}

pub(super) fn default_grid_catalog() -> Vec<GridSizePreset>
{
    let settings: GerberViewerSettings = toml::from_str(include_str!(
        "../../../../assets/gerber-viewer/default-settings.toml"
    ))
    .expect("bundled Gerber viewer settings must parse");
    settings.grid_sizes
}

pub(super) fn load_grid_catalog() -> Vec<GridSizePreset>
{
    grid_settings_path()
        .and_then(|path| load_grid_catalog_from(&path).ok())
        .filter(|catalog| !catalog.is_empty())
        .unwrap_or_else(default_grid_catalog)
}

pub(super) fn persist_grid_catalog(catalog: &[GridSizePreset]) -> Result<(), String>
{
    let path = grid_settings_path()
        .ok_or_else(|| "No operating-system configuration directory is available.".to_owned())?;
    persist_grid_catalog_to(&path, catalog)
}

pub(super) fn create_grid_definition(
    name: &str,
    x: &str,
    y: &str,
    unit: GridUnit,
    decimal_separator: &str,
) -> Result<GridSizePreset, String>
{
    let x = parse_distance(x, decimal_separator)
        .ok_or_else(|| "X must be a finite number greater than zero.".to_owned())?;
    if x <= 0.0
    {
        return Err("X must be a finite number greater than zero.".to_owned());
    }

    let y = parse_distance(y, decimal_separator)
        .ok_or_else(|| "Y must be a finite non-negative number.".to_owned())?;
    if y < 0.0
    {
        return Err("Y must be a finite non-negative number.".to_owned());
    }

    let name = name.trim();
    Ok(GridSizePreset {
        name: (!name.is_empty()).then(|| name.to_owned()),
        x,
        y,
        unit,
    })
}

pub(super) fn grid_size_choices(
    catalog: &[GridSizePreset],
    decimal_separator: &str,
) -> Vec<GridSizeChoice>
{
    catalog
        .iter()
        .enumerate()
        .map(|(index, grid)| GridSizeChoice {
            index,
            label: grid.display_label(decimal_separator),
        })
        .collect()
}

pub(super) fn system_decimal_separator() -> String
{
    platform_decimal_separator()
        .unwrap_or_else(|| DEFAULT_DECIMAL_SEPARATOR.to_owned())
}

#[cfg(windows)]
fn platform_decimal_separator() -> Option<String>
{
    use windows_sys::Win32::Globalization::{
        GetLocaleInfoEx, LOCALE_SDECIMAL,
    };

    let mut buffer = [0_u16; 8];
    let length = unsafe {
        GetLocaleInfoEx(
            std::ptr::null(),
            LOCALE_SDECIMAL,
            buffer.as_mut_ptr(),
            buffer.len() as i32,
        )
    };
    if length <= 1
    {
        return None;
    }

    String::from_utf16(&buffer[..length as usize - 1])
        .ok()
        .filter(|separator| !separator.is_empty())
}

#[cfg(unix)]
fn platform_decimal_separator() -> Option<String>
{
    let output = std::process::Command::new("locale")
        .arg("decimal_point")
        .output()
        .ok()?;
    if !output.status.success()
    {
        return None;
    }

    let separator = String::from_utf8(output.stdout)
        .ok()?
        .trim()
        .trim_matches('"')
        .to_owned();
    (!separator.is_empty()).then_some(separator)
}

#[cfg(not(any(unix, windows)))]
fn platform_decimal_separator() -> Option<String>
{
    None
}

fn format_dimensions(
    x: f64,
    y: f64,
    decimals: usize,
    unit: &str,
    decimal_separator: &str,
) -> String
{
    if x == y
    {
        format!(
            "{} {unit}",
            format_decimal(x, decimals, decimal_separator),
        )
    }
    else
    {
        format!(
            "{} {unit} ⨯ {} {unit}",
            format_decimal(x, decimals, decimal_separator),
            format_decimal(y, decimals, decimal_separator),
        )
    }
}

fn format_decimal(value: f64, decimals: usize, decimal_separator: &str) -> String
{
    format!("{value:.decimals$}").replace('.', decimal_separator)
}

fn parse_distance(value: &str, decimal_separator: &str) -> Option<f64>
{
    let value = value.trim();
    let normalized = if decimal_separator == DEFAULT_DECIMAL_SEPARATOR
    {
        value.to_owned()
    }
    else
    {
        value.replace(decimal_separator, DEFAULT_DECIMAL_SEPARATOR)
    };
    normalized
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
}

fn grid_settings_path() -> Option<PathBuf>
{
    crate::config_root::config_root().map(|root| root.join(SETTINGS_FILE_NAME))
}

fn load_grid_catalog_from(path: &Path) -> Result<Vec<GridSizePreset>, String>
{
    let source = std::fs::read_to_string(path)
        .map_err(|error| format!("Could not read {}: {error}", path.display()))?;
    let settings: GerberViewerSettings = toml::from_str(&source)
        .map_err(|error| format!("Could not parse {}: {error}", path.display()))?;
    if settings.grid_sizes.iter().any(|grid| {
        !grid.x.is_finite()
            || grid.x <= 0.0
            || !grid.y.is_finite()
            || grid.y < 0.0
    })
    {
        return Err("The persisted grid catalog contains invalid distances.".to_owned());
    }
    Ok(settings.grid_sizes)
}

fn persist_grid_catalog_to(path: &Path, catalog: &[GridSizePreset]) -> Result<(), String>
{
    let settings = GerberViewerSettings {
        grid_sizes: catalog.to_vec(),
    };
    let source = toml::to_string_pretty(&settings)
        .map_err(|error| format!("Could not serialize Gerber viewer settings: {error}"))?;
    signex_types::atomic_io::atomic_write(path, source.as_bytes())
        .map_err(|error| format!("Could not save {}: {error}", path.display()))
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn bundled_catalog_has_the_required_order_and_labels()
    {
        let catalog = default_grid_catalog();
        let labels = catalog
            .iter()
            .map(|grid| grid.display_label(","))
            .collect::<Vec<_>>();

        assert_eq!(catalog.len(), 22);
        assert_eq!(
            labels,
            [
                "100,00 mils (2,5400 mm)",
                "50,00 mils (1,2700 mm)",
                "25,00 mils (0,6350 mm)",
                "20,00 mils (0,5080 mm)",
                "10,00 mils (0,2540 mm)",
                "5,00 mils (0,1270 mm)",
                "2,50 mils (0,0635 mm)",
                "2,00 mils (0,0508 mm)",
                "1,00 mils (0,0254 mm)",
                "0,50 mils (0,0127 mm)",
                "0,20 mils (0,0051 mm)",
                "0,10 mils (0,0025 mm)",
                "5,0000 mm (196,85 mils)",
                "1,5000 mm ⨯ 2,5000 mm (59,06 mils ⨯ 98,43 mils)",
                "1,0000 mm (39,37 mils)",
                "0,5000 mm (19,69 mils)",
                "0,2500 mm (9,84 mils)",
                "0,2000 mm (7,87 mils)",
                "0,1000 mm (3,94 mils)",
                "0,0500 mm ⨯ 0,0000 mm (1,97 mils ⨯ 0,00 mils)",
                "0,0250 mm ⨯ 0,0000 mm (0,98 mils ⨯ 0,00 mils)",
                "0,0100 mm ⨯ 0,0000 mm (0,39 mils ⨯ 0,00 mils)",
            ]
        );
    }

    #[test]
    fn metric_presets_keep_exact_metric_spacing()
    {
        let catalog = default_grid_catalog();
        let rectangular = &catalog[13];
        let x_only = &catalog[19];

        assert_eq!(rectangular.x_millimetres(), 1.5);
        assert_eq!(rectangular.y_millimetres(), 2.5);
        assert_eq!(x_only.x_millimetres(), 0.05);
        assert_eq!(x_only.y_millimetres(), 0.0);
    }

    #[test]
    fn labels_use_the_supplied_decimal_separator()
    {
        let catalog = default_grid_catalog();

        assert_eq!(
            catalog[0].display_label("."),
            "100.00 mils (2.5400 mm)"
        );
        assert_eq!(
            catalog[21].display_label("."),
            "0.0100 mm ⨯ 0.0000 mm (0.39 mils ⨯ 0.00 mils)"
        );
    }

    #[test]
    fn operating_system_decimal_separator_is_available()
    {
        assert_eq!(DEFAULT_DECIMAL_SEPARATOR, ".");
        assert!(!system_decimal_separator().is_empty());
    }

    #[test]
    fn creates_named_and_unnamed_grid_definitions()
    {
        let named = create_grid_definition(
            " Fine metric ",
            "0,05",
            "0",
            GridUnit::Mm,
            ",",
        )
        .expect("valid named metric grid");
        let unnamed = create_grid_definition(
            "  ",
            "2.5",
            "2.5",
            GridUnit::Mil,
            ".",
        )
        .expect("valid unnamed mil grid");

        assert_eq!(named.name.as_deref(), Some("Fine metric"));
        assert_eq!(named.x, 0.05);
        assert_eq!(named.y, 0.0);
        assert_eq!(named.unit, GridUnit::Mm);
        assert_eq!(unnamed.name, None);
    }

    #[test]
    fn rejects_invalid_grid_distances()
    {
        for x in ["", "0", "-1", "NaN", "inf"]
        {
            assert!(
                create_grid_definition("", x, "1", GridUnit::Mm, ".")
                    .is_err()
            );
        }
        for y in ["", "-1", "NaN", "inf"]
        {
            assert!(
                create_grid_definition("", "1", y, GridUnit::Mm, ".")
                    .is_err()
            );
        }
    }

    #[test]
    fn persists_and_loads_the_ordered_grid_catalog()
    {
        let directory = tempfile::tempdir().expect("temporary settings directory");
        let path = directory.path().join("gerber_viewer.toml");
        let mut catalog = default_grid_catalog();
        catalog.push(
            create_grid_definition(
                "Assembly",
                "0.25",
                "0.5",
                GridUnit::Mm,
                ".",
            )
            .expect("valid custom grid"),
        );

        persist_grid_catalog_to(&path, &catalog)
            .expect("grid catalog must persist");
        let loaded = load_grid_catalog_from(&path)
            .expect("persisted grid catalog must load");

        assert_eq!(loaded, catalog);
        assert_eq!(loaded.last().and_then(|grid| grid.name.as_deref()), Some("Assembly"));
    }
}
