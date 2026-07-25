use std::collections::BTreeSet;

use super::*;

const COMPONENT_HIGHLIGHT_COLOR: Color = Color::from_rgb(
    1.0,
    193.0 / 255.0,
    7.0 / 255.0,
);
const NON_MATCHING_ALPHA: f32 = 0.18;

impl GerberViewerState
{
    pub fn component_choices(&self) -> Vec<String>
    {
        self.layers
            .iter()
            .flat_map(|viewer_layer| {
                viewer_layer
                    .layer
                    .geometry
                    .primitive_attributes
                    .iter()
            })
            .filter_map(|attributes| attributes.component.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn highlighted_component(&self) -> Option<&str>
    {
        self.highlighted_component.as_deref()
    }

    pub fn set_highlighted_component(&mut self, component: String)
    {
        if self
            .component_choices()
            .iter()
            .any(|candidate| candidate == &component)
        {
            self.status = format!("Highlighted component: {component}");
            self.highlighted_component = Some(component);
            self.highlighted_net = None;
            self.highlighted_attribute = None;
            self.redraw_generation = self.redraw_generation.wrapping_add(1);
        }
    }

    pub fn clear_component_highlight(&mut self)
    {
        if self.highlighted_component.take().is_some()
        {
            self.status = "Component highlight cleared.".into();
            self.redraw_generation = self.redraw_generation.wrapping_add(1);
        }
    }

    pub(super) fn retain_available_component_highlight(&mut self)
    {
        let Some(component) = self.highlighted_component.as_deref() else
        {
            return;
        };
        if !self
            .component_choices()
            .iter()
            .any(|candidate| candidate == component)
        {
            self.highlighted_component = None;
        }
    }

    pub fn net_choices(&self) -> Vec<String>
    {
        self.layers
            .iter()
            .flat_map(|viewer_layer| {
                viewer_layer
                    .layer
                    .geometry
                    .primitive_attributes
                    .iter()
            })
            .flat_map(|attributes| attributes.nets.iter().cloned())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn highlighted_net(&self) -> Option<&str>
    {
        self.highlighted_net.as_deref()
    }

    pub fn set_highlighted_net(&mut self, net: String)
    {
        if self.net_choices().iter().any(|candidate| candidate == &net)
        {
            self.status = format!("Highlighted net: {net}");
            self.highlighted_net = Some(net);
            self.highlighted_component = None;
            self.highlighted_attribute = None;
            self.redraw_generation = self.redraw_generation.wrapping_add(1);
        }
    }

    pub fn clear_net_highlight(&mut self)
    {
        if self.highlighted_net.take().is_some()
        {
            self.status = "Net highlight cleared.".into();
            self.redraw_generation = self.redraw_generation.wrapping_add(1);
        }
    }

    pub(super) fn retain_available_net_highlight(&mut self)
    {
        let Some(net) = self.highlighted_net.as_deref() else
        {
            return;
        };
        if !self.net_choices().iter().any(|candidate| candidate == net)
        {
            self.highlighted_net = None;
        }
    }

    pub fn attribute_choices(&self) -> Vec<GerberAttributeValue>
    {
        self.layers
            .iter()
            .flat_map(|viewer_layer| {
                viewer_layer
                    .layer
                    .geometry
                    .primitive_attributes
                    .iter()
            })
            .flat_map(|attributes| attributes.attributes.iter().cloned())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn highlighted_attribute(&self) -> Option<&GerberAttributeValue>
    {
        self.highlighted_attribute.as_ref()
    }

    pub fn set_highlighted_attribute(&mut self, attribute: GerberAttributeValue)
    {
        if self
            .attribute_choices()
            .iter()
            .any(|candidate| candidate == &attribute)
        {
            self.status = format!("Highlighted attribute: {attribute}");
            self.highlighted_attribute = Some(attribute);
            self.highlighted_component = None;
            self.highlighted_net = None;
            self.redraw_generation = self.redraw_generation.wrapping_add(1);
        }
    }

    pub fn clear_attribute_highlight(&mut self)
    {
        if self.highlighted_attribute.take().is_some()
        {
            self.status = "Attribute highlight cleared.".into();
            self.redraw_generation = self.redraw_generation.wrapping_add(1);
        }
    }

    pub(super) fn retain_available_attribute_highlight(&mut self)
    {
        let Some(attribute) = self.highlighted_attribute.as_ref() else
        {
            return;
        };
        if !self
            .attribute_choices()
            .iter()
            .any(|candidate| candidate == attribute)
        {
            self.highlighted_attribute = None;
        }
    }
}

pub(super) fn attribute_highlight_color(
    layer_color: Color,
    attributes: Option<&signex_gerber::GerberObjectAttributes>,
    highlighted_attribute: Option<&GerberAttributeValue>,
) -> Color
{
    let Some(highlighted_attribute) = highlighted_attribute else
    {
        return layer_color;
    };
    if attributes.is_some_and(|attributes| {
        attributes
            .attributes
            .iter()
            .any(|attribute| attribute == highlighted_attribute)
    })
    {
        COMPONENT_HIGHLIGHT_COLOR
    }
    else
    {
        Color {
            a: NON_MATCHING_ALPHA,
            ..layer_color
        }
    }
}

pub(super) fn net_highlight_color(
    layer_color: Color,
    attributes: Option<&signex_gerber::GerberObjectAttributes>,
    highlighted_net: Option<&str>,
) -> Color
{
    let Some(highlighted_net) = highlighted_net else
    {
        return layer_color;
    };
    if attributes.is_some_and(|attributes| {
        attributes.nets.iter().any(|net| net == highlighted_net)
    })
    {
        COMPONENT_HIGHLIGHT_COLOR
    }
    else
    {
        Color {
            a: NON_MATCHING_ALPHA,
            ..layer_color
        }
    }
}

pub(super) fn component_highlight_color(
    layer_color: Color,
    attributes: Option<&signex_gerber::GerberObjectAttributes>,
    highlighted_component: Option<&str>,
) -> Color
{
    let Some(highlighted_component) = highlighted_component else
    {
        return layer_color;
    };
    if attributes
        .and_then(|attributes| attributes.component.as_deref())
        == Some(highlighted_component)
    {
        COMPONENT_HIGHLIGHT_COLOR
    }
    else
    {
        Color {
            a: NON_MATCHING_ALPHA,
            ..layer_color
        }
    }
}

#[cfg(test)]
#[path = "../../tests/gerber_viewer/highlight.rs"]
mod gerber_highlight_test_definitions;

#[cfg(test)]
gerber_highlight_test_definitions::gerber_highlight_tests!();

#[cfg(test)]
#[path = "../../tests/gerber_viewer/net_highlight.rs"]
mod gerber_net_highlight_test_definitions;

#[cfg(test)]
gerber_net_highlight_test_definitions::gerber_net_highlight_tests!();

#[cfg(test)]
#[path = "../../tests/gerber_viewer/attribute_highlight.rs"]
mod gerber_attribute_highlight_test_definitions;

#[cfg(test)]
gerber_attribute_highlight_test_definitions::gerber_attribute_highlight_tests!();
