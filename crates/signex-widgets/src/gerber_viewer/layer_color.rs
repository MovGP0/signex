use std::fmt;

use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct GerberLayerColorChoice
{
    pub(super) palette_index: usize,
    label: String,
}

impl fmt::Display for GerberLayerColorChoice
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        formatter.write_str(&self.label)
    }
}

impl GerberViewerState
{
    pub(super) fn layer_color_choices(&self) -> Vec<GerberLayerColorChoice>
    {
        self.palette
            .iter()
            .enumerate()
            .filter(|(palette_index, color)|
            {
                !self.palette[..*palette_index].contains(color)
            })
            .map(|(palette_index, color)|
            {
                GerberLayerColorChoice {
                    palette_index,
                    label: format!(
                        "Material {:02} · {}",
                        palette_index + 1,
                        color_hex(*color),
                    ),
                }
            })
            .collect()
    }

    pub(super) fn selected_layer_color_choice(
        &self,
        layer_index: usize,
    ) -> Option<GerberLayerColorChoice>
    {
        let color = self.layers.get(layer_index)?.color;
        let palette_index = self
            .palette
            .iter()
            .position(|candidate| *candidate == color)?;
        self.layer_color_choices()
            .into_iter()
            .find(|choice| choice.palette_index == palette_index)
    }

    pub(super) fn selected_color_choice(
        &self,
        color: Color,
    ) -> Option<GerberLayerColorChoice>
    {
        let palette_index = self
            .palette
            .iter()
            .position(|candidate| *candidate == color)?;
        self.layer_color_choices()
            .into_iter()
            .find(|choice| choice.palette_index == palette_index)
    }

    pub fn set_layer_color(&mut self, layer_index: usize, palette_index: usize)
    {
        let Some(color) = self.palette.get(palette_index).copied() else
        {
            return;
        };
        let Some(layer) = self.layers.get_mut(layer_index) else
        {
            return;
        };
        if layer.color == color
        {
            return;
        }

        layer.color = color;
        self.redraw_generation = self.redraw_generation.wrapping_add(1);
        self.status = format!(
            "Layer color: {} {}",
            layer.layer.name,
            color_hex(color),
        );
    }
}

fn color_hex(color: Color) -> String
{
    format!(
        "#{:02X}{:02X}{:02X}",
        (color.r * 255.0).round() as u8,
        (color.g * 255.0).round() as u8,
        (color.b * 255.0).round() as u8,
    )
}
