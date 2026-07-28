//! Home of the Command Registry (#278). Slice 1 lands only the
//! id→[`crate::app::Message`] bridge; the registry struct, dispatch,
//! args, and enablement are later slices.

pub mod bridge;

pub use bridge::core_to_message;
