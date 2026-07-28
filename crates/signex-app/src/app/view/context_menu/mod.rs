//! Right-click / context-menu views: the reusable menu-item builders
//! (`items`), the concrete menus (`menus`), and the submenu launcher
//! (`submenu`). The folder carries the `context_menu` namespace; the
//! `impl Signex` builders here are called from `view/mod.rs` and its
//! sibling `overlays` module.

mod items;
mod menus;
mod project_tree;
mod submenu;

#[cfg(test)]
mod tests;

use super::{
    ContextAction, ContextMenuMsg, ContextSubmenu, Element, Length, Message, OverlayMsg,
    PreferencesMsg, SUBMENU_ARROW, SUBMENU_ARROW_SIZE, Signex, UiMsg, container, menu_bar,
};
