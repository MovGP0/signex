//! Gerber and Excellon viewer-domain support.
//!
//! File parsing and editing are delegated to `lib_gerber_edit`. This crate
//! translates the parsed representation into stable, renderer-independent
//! geometry consumed by the Signex application.

mod geometry;
mod loading;

pub use geometry::{
    ApertureShape, Bounds, GerberGeometry, GerberPrimitive, Point, PrimitivePolarity,
};
pub use lib_gerber_edit::layer::LayerType;
pub use loading::{
    GerberLoadBatch, GerberLoadFailure, LoadedLayer, load_excellon_file, load_excellon_files,
    load_excellon_reader, load_gerber_file, load_gerber_files, load_gerber_reader,
};

#[cfg(test)]
mod geometry_tests;
#[cfg(test)]
mod loading_tests;
