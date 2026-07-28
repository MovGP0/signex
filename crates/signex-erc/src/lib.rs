//! Electrical Rules Check. Runs on a [`SchematicSheet`] and returns a
//! list of [`Violation`]s. Internally the sheet is projected into an
//! [`ErcContext`] first, so rule logic stays independent from renderer internals.
//!
//! # Architecture
//!
//! ```text
//! SchematicSheet
//!        ↓  (projection)
//!    ErcContext
//!        ↓  (engine::run_all)
//!  Vec<Diagnostic>
//!        ↓  (From<Diagnostic>)
//!  Vec<Violation>   ← public API
//! ```
//!
//! ERC reads from `signex_types::SchematicSheet` directly.

use serde::{Deserialize, Serialize};
use signex_types::schematic::{Point, SchematicSheet, SelectedItem, SelectedKind};

pub mod context;
pub mod diagnostic;
pub mod engine;
pub mod rule;
mod rules;

pub use context::ErcContext;
pub use diagnostic::Diagnostic;
pub use rule::{AnalysisScope, Applicability, RuleDefinition, RuleId, RuleTarget};

/// What kind of violation this is. Stable identifier that maps to a severity
/// in the user's configuration. Ordered by the Altium ERC matrix conventions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RuleKind {
    /// A pin has no connected wire, no NC, and no matching pin on another part.
    UnusedPin,
    /// Two symbols share the same non-blank reference designator.
    DuplicateRefDesignator,
    /// A hierarchical port / label has no connected wire or bus segment.
    HierPortDisconnected,
    /// A wire endpoint dangles — no matching pin, junction, label, or port.
    DanglingWire,
    /// A net name appears on incompatible label types (e.g. a local label and
    /// a global label both carrying different nets).
    NetLabelConflict,
    /// A label sits in free space — neither on a wire endpoint nor a pin.
    OrphanLabel,
    /// A bus segment connects two buses whose bit-widths differ.
    BusBitWidthMismatch,
    /// A sheet symbol's port pin doesn't match a label/port on the child sheet.
    BadHierSheetPin,
    /// A net containing a power pin lacks a power flag (`PWR_FLAG`).
    MissingPowerFlag,
    /// Two power ports with incompatible nets are shorted together.
    PowerPortShort,
    /// A symbol or wire sits outside the active sheet's page boundary.
    SymbolOutsideSheet,
    /// A label sits where two or more wires cross with no junction. The
    /// netlist anchors it to exactly one of them by a deterministic tiebreak
    /// (issue #402) — a defensible runtime answer, but not one the user can
    /// predict from the geometry, so it is surfaced rather than assumed.
    AmbiguousLabelAnchor,
}

impl RuleKind {
    /// Human-readable rule name for the Messages panel and preferences UI.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::UnusedPin => "Unused pin",
            Self::DuplicateRefDesignator => "Duplicate reference designator",
            Self::HierPortDisconnected => "Hierarchical port disconnected",
            Self::DanglingWire => "Dangling wire endpoint",
            Self::NetLabelConflict => "Net label conflict",
            Self::OrphanLabel => "Orphan label",
            Self::BusBitWidthMismatch => "Bus bit-width mismatch",
            Self::BadHierSheetPin => "Bad hierarchical sheet pin",
            Self::MissingPowerFlag => "Missing power flag",
            Self::PowerPortShort => "Power port short",
            Self::SymbolOutsideSheet => "Symbol outside sheet boundary",
            Self::AmbiguousLabelAnchor => "Ambiguous label anchor",
        }
    }

    /// Default severity. Users can override per-rule in the Preferences
    /// panel via `ui_state.erc_severity_override`.
    #[must_use]
    pub const fn default_severity(self) -> Severity {
        match self {
            Self::DuplicateRefDesignator
            | Self::BusBitWidthMismatch
            | Self::BadHierSheetPin
            | Self::PowerPortShort => Severity::Error,
            Self::UnusedPin
            | Self::HierPortDisconnected
            | Self::DanglingWire
            | Self::NetLabelConflict
            | Self::OrphanLabel
            | Self::MissingPowerFlag
            | Self::AmbiguousLabelAnchor => Severity::Warning,
            Self::SymbolOutsideSheet => Severity::Info,
        }
    }
}

/// Severity level — maps to colours and sort order in the Messages panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Severity {
    Error,
    Warning,
    Info,
    /// Rule is disabled — emit nothing.
    Off,
}

/// A concrete violation emitted by a rule run. `location` is in world-space
/// mm; the app uses it to centre the canvas when the user clicks the message.
#[derive(Debug, Clone)]
pub struct Violation {
    pub rule: RuleKind,
    pub severity: Severity,
    pub message: String,
    pub location: Point,
    /// The primary object the violation refers to.
    pub primary: Option<SelectedItem>,
    /// Optional peer — the "other" object in two-object rules.
    pub peer: Option<SelectedItem>,
}

/// Run every enabled rule against the snapshot. Returns a flat list of
/// violations in rule order.
pub fn run(snapshot: &SchematicSheet) -> Vec<Violation> {
    let ctx = ErcContext::from_snapshot(snapshot);
    engine::run_all(&ctx)
        .into_iter()
        .map(Violation::from)
        .collect()
}

/// Run built-in ERC rules plus caller-provided DSL evaluator functions.
pub fn run_with_dsl(snapshot: &SchematicSheet, dsl_rules: &[engine::EvalFn]) -> Vec<Violation> {
    let ctx = ErcContext::from_snapshot(snapshot);
    engine::run_all_with_dsl(&ctx, dsl_rules)
        .into_iter()
        .map(Violation::from)
        .collect()
}

/// Run ERC for a schematic in the context of a whole project.
///
/// Cross-sheet rules consult `children` keyed by the child's filename as it appears
/// on the parent's sheet symbol. Pass an empty map for top-only runs.
#[expect(
    clippy::implicit_hasher,
    reason = "project snapshots use the standard HashMap throughout the ERC API"
)]
pub fn run_with_project(
    snapshot: &SchematicSheet,
    children: &std::collections::HashMap<String, SchematicSheet>,
) -> Vec<Violation> {
    let ctx = ErcContext::from_snapshot_with_children(snapshot, children);
    engine::run_all(&ctx)
        .into_iter()
        .map(Violation::from)
        .collect()
}

/// Run project-scoped ERC with built-in rules plus caller-provided DSL rules.
#[expect(
    clippy::implicit_hasher,
    reason = "project snapshots use the standard HashMap throughout the ERC API"
)]
pub fn run_with_project_and_dsl(
    snapshot: &SchematicSheet,
    children: &std::collections::HashMap<String, SchematicSheet>,
    dsl_rules: &[engine::EvalFn],
) -> Vec<Violation> {
    let ctx = ErcContext::from_snapshot_with_children(snapshot, children);
    engine::run_all_with_dsl(&ctx, dsl_rules)
        .into_iter()
        .map(Violation::from)
        .collect()
}

/// Helper for rules to build a [`SelectedItem`] when they only have a uuid.
pub(crate) const fn sel(uuid: uuid::Uuid, kind: SelectedKind) -> SelectedItem {
    SelectedItem::new(uuid, kind)
}
