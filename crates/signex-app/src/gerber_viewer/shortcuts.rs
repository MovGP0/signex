use super::*;

pub(super) fn next_layer_index(active_layer: Option<usize>, layer_count: usize) -> Option<usize>
{
    if layer_count == 0
    {
        return None;
    }

    match active_layer
    {
        Some(index) if index + 1 < layer_count => Some(index + 1),
        None => Some(0),
        _ => None,
    }
}
pub(super) fn previous_layer_index(active_layer: Option<usize>, layer_count: usize) -> Option<usize>
{
    if layer_count == 0
    {
        return None;
    }

    match active_layer
    {
        Some(index) if index > 0 && index < layer_count => Some(index - 1),
        None => Some(layer_count - 1),
        _ => None,
    }
}
pub(super) fn gerber_shortcut_message(
    keymap: &crate::keymap::CompiledKeymap,
    key: &keyboard::Key,
    modifiers: keyboard::Modifiers,
) -> Option<GerberViewerMessage>
{
    let stroke = crate::keymap::KeyStroke::from_iced(key, modifiers)?;
    let lookup = keymap.lookup(
        &[stroke],
        &[
            crate::keymap::ShortcutContext::Global,
            crate::keymap::ShortcutContext::Gerber,
        ],
    );
    match lookup.command?.as_str()
    {
        "gerber_next_layer" => Some(GerberViewerMessage::NextLayer),
        "gerber_previous_layer" => Some(GerberViewerMessage::PreviousLayer),
        _ => None,
    }
}

