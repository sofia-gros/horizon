//! Bounded declarative definitions for server-authoritative mod menus.
//! A definition is data, not authority to mutate a player's inventory.
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Gui {
    pub width: u16,
    pub height: u16,
    pub slots: Vec<Slot>,
    /// Position of the top-left player inventory slot; absent preserves the legacy layout.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_inventory: Option<PlayerInventory>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scroll: Option<ScrollGrid>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub shared_selection: bool,
    #[serde(default)]
    pub elements: Vec<Element>,
}

/// A bounded row-scrolling viewport for existing buttons, in protocol order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScrollGrid {
    pub x: u16,
    pub y: u16,
    pub columns: u8,
    pub rows: u8,
    pub cell_width: u16,
    pub cell_height: u16,
    pub buttons: Vec<String>,
    pub bar_x: u16,
    pub bar_y: u16,
    pub bar_height: u16,
    pub handle_width: u16,
    pub handle_height: u16,
    pub handle: ButtonImages,
}
impl ScrollGrid {
    pub fn max_row(&self) -> usize {
        self.buttons
            .len()
            .div_ceil(usize::from(self.columns))
            .saturating_sub(usize::from(self.rows))
    }
    pub fn row_after_scroll(&self, current: usize, delta: f32) -> usize {
        ((current as i32) - delta.signum() as i32).clamp(0, self.max_row() as i32) as usize
    }
    pub fn button_position(&self, id: &str, row: usize) -> Option<(u16, u16)> {
        let index = self.buttons.iter().position(|button| button == id)?;
        let first = row.min(self.max_row()) * usize::from(self.columns);
        let relative = index.checked_sub(first)?;
        if relative >= usize::from(self.columns) * usize::from(self.rows) {
            return None;
        }
        Some((
            self.x + (relative % usize::from(self.columns)) as u16 * self.cell_width,
            self.y + (relative / usize::from(self.columns)) as u16 * self.cell_height,
        ))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerInventory {
    pub x: u16,
    pub y: u16,
    #[serde(default = "enabled_by_default", skip_serializing_if = "is_enabled")]
    pub background: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Slot {
    pub id: String,
    pub x: u16,
    pub y: u16,
    pub role: SlotRole,
    #[serde(default = "enabled_by_default", skip_serializing_if = "is_enabled")]
    pub background: bool,
    #[serde(default)]
    pub accepts: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SlotRole {
    Input,
    Output,
    Storage,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Element {
    Label {
        x: u16,
        y: u16,
        text: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        color: Option<[u8; 3]>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        when_selected: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        slot: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        matches: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        excludes: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        scale: Option<u8>,
    },
    Image {
        x: u16,
        y: u16,
        width: u16,
        height: u16,
        texture: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        region: Option<[u16; 4]>,
    },
    Button {
        id: String,
        x: u16,
        y: u16,
        width: u16,
        height: u16,
        text: String,
        #[serde(default = "enabled_by_default", skip_serializing_if = "is_enabled")]
        enabled: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        images: Option<ButtonImages>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        icon: Option<String>,
    },
    Progress {
        id: String,
        x: u16,
        y: u16,
        width: u16,
        height: u16,
        #[serde(default)]
        value: u8,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ButtonImages {
    pub normal: String,
    #[serde(default)]
    pub hover: Option<String>,
    #[serde(default)]
    pub selected: Option<String>,
    #[serde(default)]
    pub disabled: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regions: Option<ButtonRegions>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ButtonRegions {
    pub normal: [u16; 4],
    #[serde(default)]
    pub hover: Option<[u16; 4]>,
    #[serde(default)]
    pub selected: Option<[u16; 4]>,
    #[serde(default)]
    pub disabled: Option<[u16; 4]>,
}
impl ButtonRegions {
    pub fn region(&self, enabled: bool, selected: bool, hovered: bool) -> [u16; 4] {
        if !enabled {
            self.disabled
        } else if selected {
            self.selected
        } else if hovered {
            self.hover
        } else {
            None
        }
        .unwrap_or(self.normal)
    }
    fn valid(&self) -> bool {
        region_valid(self.normal)
            && [self.hover, self.selected, self.disabled]
                .into_iter()
                .flatten()
                .all(region_valid)
    }
}
fn region_valid([x, y, w, h]: [u16; 4]) -> bool {
    w > 0 && h > 0 && u32::from(x) + u32::from(w) <= 1024 && u32::from(y) + u32::from(h) <= 512
}
impl ButtonImages {
    /// Resolve the visible image, falling back to the normal image for missing states.
    pub fn texture(&self, enabled: bool, selected: bool, hovered: bool) -> &str {
        if !enabled {
            self.disabled.as_deref()
        } else if selected {
            self.selected.as_deref()
        } else if hovered {
            self.hover.as_deref()
        } else {
            None
        }
        .unwrap_or(&self.normal)
    }
}

fn enabled_by_default() -> bool {
    true
}
fn is_false(value: &bool) -> bool {
    !*value
}
fn is_enabled(enabled: &bool) -> bool {
    *enabled
}
fn image_valid(value: &str) -> bool {
    value.len() <= 128
        && !value.starts_with('#')
        && resource_valid(value)
        && value.split_once(':').is_some_and(|(_, path)| {
            path.starts_with("gui/")
                && !path
                    .split('/')
                    .any(|s| s.is_empty() || s == "." || s == "..")
        })
}

fn id_valid(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"_.-".contains(&c))
}
fn resource_valid(value: &str) -> bool {
    let value = value.strip_prefix('#').unwrap_or(value);
    value.split_once(':').is_some_and(|(ns, path)| {
        !ns.is_empty()
            && !path.is_empty()
            && ns
                .bytes()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_.-".contains(&c))
            && path
                .bytes()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_./-".contains(&c))
    })
}
impl Gui {
    /// Parse untrusted mod data with explicit memory, geometry and identity bounds.
    pub fn parse(source: &str) -> Result<Self, String> {
        if source.len() > 16384 {
            return Err("GUI definition too large".into());
        }
        let gui: Self = serde_json::from_str(source).map_err(|e| e.to_string())?;
        gui.validate()?;
        Ok(gui)
    }
    /// Top-left of the main player inventory's first slot in GUI coordinates.
    pub fn player_inventory_origin(&self) -> (u16, u16) {
        self.player_inventory
            .as_ref()
            .map_or(((self.width - 176) / 2 + 8, self.height - 82), |p| {
                (p.x, p.y)
            })
    }
    /// Return the protocol button index and id at an enabled button's GUI-local position.
    pub fn button_at(&self, x: f32, y: f32) -> Option<(usize, &str)> {
        self.button_at_row(x, y, 0)
    }
    pub fn button_position(&self, id: &str, x: u16, y: u16, row: usize) -> Option<(u16, u16)> {
        match &self.scroll {
            Some(scroll) if scroll.buttons.iter().any(|button| button == id) => {
                scroll.button_position(id, row)
            }
            _ => Some((x, y)),
        }
    }
    pub fn button_at_row(&self, x: f32, y: f32, row: usize) -> Option<(usize, &str)> {
        self.elements
            .iter()
            .filter_map(|element| match element {
                Element::Button {
                    id,
                    x,
                    y,
                    width,
                    height,
                    enabled,
                    ..
                } => Some((id, *x, *y, *width, *height, *enabled)),
                _ => None,
            })
            .enumerate()
            .find_map(|(index, (id, bx, by, width, height, enabled))| {
                let (bx, by) = self.button_position(id, bx, by, row)?;
                (enabled
                    && x >= f32::from(bx)
                    && y >= f32::from(by)
                    && x < f32::from(bx) + f32::from(width)
                    && y < f32::from(by) + f32::from(height))
                .then_some((index, id.as_str()))
            })
    }
    /// Decode the server's per-block selection; zero means no selection.
    pub fn selection_from_code(&self, code: u8) -> Option<&str> {
        let index = usize::from(code).checked_sub(1)?;
        self.elements
            .iter()
            .filter_map(|element| match element {
                Element::Button {
                    id, enabled: true, ..
                } => Some(Some(id.as_str())),
                Element::Button { .. } => Some(None),
                _ => None,
            })
            .nth(index)
            .flatten()
    }
    pub fn selection_code(&self, id: &str) -> Option<u8> {
        let index = self
            .elements
            .iter()
            .filter_map(|element| match element {
                Element::Button {
                    id: button,
                    enabled,
                    ..
                } => Some((button, enabled)),
                _ => None,
            })
            .position(|(button, enabled)| *enabled && button == id)?;
        u8::try_from(index + 1).ok()
    }
    pub fn validate(&self) -> Result<(), String> {
        if !(176..=512).contains(&self.width)
            || !(166..=512).contains(&self.height)
            || self.slots.len() > 54
            || self.elements.len() > 64
        {
            return Err("GUI dimensions or element count out of range".into());
        }
        let inside = |x: u16, y: u16, w: u16, h: u16| {
            w > 0
                && h > 0
                && u32::from(x) + u32::from(w) <= u32::from(self.width)
                && u32::from(y) + u32::from(h) <= u32::from(self.height)
        };
        if let Some(p) = &self.player_inventory
            && !inside(p.x, p.y, 162, 76)
        {
            return Err("invalid player inventory position".into());
        }
        if let Some(scroll) = &self.scroll {
            let viewport_w = u32::from(scroll.columns) * u32::from(scroll.cell_width);
            let viewport_h = u32::from(scroll.rows) * u32::from(scroll.cell_height);
            if scroll.columns == 0
                || scroll.rows == 0
                || scroll.buttons.is_empty()
                || scroll.buttons.len() > 64
                || viewport_w > u32::from(u16::MAX)
                || viewport_h > u32::from(u16::MAX)
                || !inside(scroll.x, scroll.y, viewport_w as u16, viewport_h as u16)
                || !inside(
                    scroll.bar_x,
                    scroll.bar_y,
                    scroll.handle_width,
                    scroll.bar_height,
                )
                || scroll.handle_height == 0
                || scroll.handle_height > scroll.bar_height
                || !image_valid(&scroll.handle.normal)
                || scroll.handle.regions.as_ref().is_some_and(|r| !r.valid())
                || [
                    &scroll.handle.hover,
                    &scroll.handle.selected,
                    &scroll.handle.disabled,
                ]
                .into_iter()
                .flatten()
                .any(|image| !image_valid(image))
            {
                return Err("invalid GUI scroll grid".into());
            }
            let mut seen = HashSet::new();
            for id in &scroll.buttons {
                if !seen.insert(id) || !self.elements.iter().any(|element| matches!(element, Element::Button { id: button, width, height, .. } if button == id && *width <= scroll.cell_width && *height <= scroll.cell_height)) {
                    return Err("invalid GUI scroll button".into());
                }
            }
        }
        let mut ids = HashSet::new();
        for slot in &self.slots {
            if !id_valid(&slot.id)
                || !ids.insert(&slot.id)
                || !inside(slot.x, slot.y, 18, 18)
                || slot.accepts.len() > 32
                || slot
                    .accepts
                    .iter()
                    .any(|s| s.len() > 128 || !resource_valid(s))
                || (slot.role == SlotRole::Output && !slot.accepts.is_empty())
            {
                return Err("invalid GUI slot".into());
            }
        }
        for element in &self.elements {
            let ok = match element {
                Element::Label { x, y, text, when_selected, slot, matches, excludes, scale, .. } => {
                    inside(*x, *y, 1, 1) && text.len() <= 256 && !text.chars().any(char::is_control)
                        && when_selected.as_ref().is_none_or(|id| self.elements.iter().any(|element| matches!(element, Element::Button { id: button, .. } if button == id)))
                        && slot.as_ref().is_none_or(|id| self.slots.iter().any(|item| item.id == *id && item.role != SlotRole::Output))
                        && (matches.is_none() && excludes.is_none() || slot.is_some())
                        && [matches, excludes].into_iter().flatten().all(|selector| selector.len() <= 128 && resource_valid(selector))
                        && scale.is_none_or(|value| (50..=100).contains(&value))
                }
                Element::Image {
                    x,
                    y,
                    width,
                    height,
                    texture,
                    region,
                } => {
                    inside(*x, *y, *width, *height)
                        && image_valid(texture)
                        && region.is_none_or(region_valid)
                }
                Element::Button {
                    id,
                    x,
                    y,
                    width,
                    height,
                    text,
                    images,
                    icon,
                    ..
                } => {
                    id_valid(id)
                        && ids.insert(id)
                        && inside(*x, *y, *width, *height)
                        && text.len() <= 128
                        && !text.chars().any(char::is_control)
                        && icon.as_deref().is_none_or(image_valid)
                        && images.as_ref().is_none_or(|images| {
                            image_valid(&images.normal)
                                && images.regions.as_ref().is_none_or(ButtonRegions::valid)
                                && [&images.hover, &images.selected, &images.disabled]
                                    .into_iter()
                                    .flatten()
                                    .all(|image| image_valid(image))
                        })
                }
                Element::Progress {
                    id,
                    x,
                    y,
                    width,
                    height,
                    value,
                } => {
                    id_valid(id)
                        && ids.insert(id)
                        && inside(*x, *y, *width, *height)
                        && *value <= 100
                }
            };
            if !ok {
                return Err("invalid GUI element".into());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn images_and_button_states_are_bounded() {
        let json = r#"{"width":176,"height":166,"slots":[],"elements":[{"type":"image","x":0,"y":0,"width":176,"height":166,"texture":"example:gui/background"},{"type":"button","id":"select","x":4,"y":4,"width":20,"height":10,"text":"","enabled":false,"images":{"normal":"example:gui/normal","hover":"example:gui/hover","selected":"example:gui/selected","disabled":"example:gui/disabled"}}]}"#;
        let gui = Gui::parse(json).unwrap();
        let Element::Button {
            enabled,
            images: Some(images),
            ..
        } = &gui.elements[1]
        else {
            panic!("button")
        };
        assert!(!enabled);
        assert_eq!(gui.selection_code("select"), None);
        assert_eq!(gui.selection_from_code(1), None);
        assert_eq!(gui.button_at(4.0, 4.0), None);
        let mut active = gui.clone();
        if let Element::Button { enabled, .. } = &mut active.elements[1] {
            *enabled = true;
        }
        assert_eq!(active.selection_code("select"), Some(1));
        assert_eq!(active.selection_from_code(1), Some("select"));
        assert_eq!(active.selection_from_code(0), None);
        assert_eq!(active.button_at(4.0, 4.0), Some((0, "select")));
        assert_eq!(active.button_at(23.99, 13.99), Some((0, "select")));
        assert_eq!(active.button_at(24.0, 14.0), None);
        assert_eq!(images.texture(true, false, false), "example:gui/normal");
        assert_eq!(images.texture(true, false, true), "example:gui/hover");
        assert_eq!(images.texture(true, true, true), "example:gui/selected");
        assert_eq!(images.texture(false, true, true), "example:gui/disabled");
        assert!(
            Gui::parse(&json.replace("example:gui/background", "example:gui/../secret")).is_err()
        );
        assert!(
            Gui::parse(&json.replace(
                "\"width\":176,\"height\":166,\"texture\"",
                "\"width\":177,\"height\":166,\"texture\""
            ))
            .is_err()
        );
    }
    #[test]
    fn regions_are_bounded_and_resolve_by_button_state() {
        let json = r#"{"width":176,"height":184,"slots":[],"player_inventory":{"x":8,"y":102,"background":false},"elements":[{"type":"image","x":0,"y":0,"width":176,"height":184,"texture":"example:gui/sheet","region":[0,0,176,184]},{"type":"button","id":"b","x":51,"y":15,"width":18,"height":18,"text":"","images":{"normal":"example:gui/sheet","regions":{"normal":[176,15,18,18],"selected":[176,33,18,18],"hover":[176,51,18,18]}}}]}"#;
        let gui = Gui::parse(json).unwrap();
        let Element::Button {
            images: Some(images),
            ..
        } = &gui.elements[1]
        else {
            panic!("button")
        };
        let regions = images.regions.as_ref().unwrap();
        assert_eq!(regions.region(true, false, true), [176, 51, 18, 18]);
        assert_eq!(regions.region(true, true, true), [176, 33, 18, 18]);
        assert_eq!(regions.region(false, false, false), [176, 15, 18, 18]);
        assert!(Gui::parse(&json.replace("[0,0,176,184]", "[1000,0,176,184]")).is_err());
        assert!(Gui::parse(&json.replace("[0,0,176,184]", "[0,0,0,184]")).is_err());
    }
    #[test]
    fn button_icons_and_context_labels_are_checked() {
        let json = r#"{"width":302,"height":184,"slots":[{"id":"material","x":29,"y":43,"role":"input"}],"elements":[{"type":"button","id":"head","x":51,"y":15,"width":18,"height":18,"text":"","icon":"example:gui/head"},{"type":"label","x":180,"y":30,"text":"Material: ","color":[255,255,255],"slot":"material","when_selected":"head"}]}"#;
        assert!(Gui::parse(json).is_ok());
        let filtered = json.replace(
            "\"slot\":\"material\"",
            "\"slot\":\"material\",\"matches\":\"#minecraft:planks\",\"scale\":78",
        );
        assert!(Gui::parse(&filtered).is_ok());
        assert!(Gui::parse(&filtered.replace("\"scale\":78", "\"scale\":49")).is_err());
        assert!(Gui::parse(&filtered.replace("#minecraft:planks", "../invalid")).is_err());
        assert!(Gui::parse(&json.replace("example:gui/head", "example:gui/../bad")).is_err());
        assert!(Gui::parse(&json.replace("\"slot\":\"material\"", "\"slot\":\"absent\"")).is_err());
        assert!(
            Gui::parse(&json.replace("\"when_selected\":\"head\"", "\"when_selected\":\"absent\""))
                .is_err()
        );
    }
    #[test]
    fn scrolling_keeps_button_indices_and_clamps_both_ends() {
        let buttons: Vec<_> = (0..20).map(|i| serde_json::json!({"type":"button", "id":format!("b{i}"), "x":0, "y":0, "width":18, "height":18, "text":""})).collect();
        let names: Vec<_> = (0..20).map(|i| format!("b{i}")).collect();
        let value = serde_json::json!({"width":176,"height":184,"slots":[],"elements":buttons,"scroll":{"x":51,"y":15,"columns":4,"rows":4,"cell_width":18,"cell_height":18,"buttons":names,"bar_x":126,"bar_y":15,"bar_height":72,"handle_width":12,"handle_height":15,"handle":{"normal":"example:gui/sheet"}}});
        let gui = Gui::parse(&value.to_string()).unwrap();
        let scroll = gui.scroll.as_ref().unwrap();
        assert_eq!(scroll.max_row(), 1);
        assert_eq!(scroll.row_after_scroll(0, 1.0), 0);
        assert_eq!(scroll.row_after_scroll(0, -1.0), 1);
        assert_eq!(scroll.row_after_scroll(1, -1.0), 1);
        assert_eq!(gui.button_at_row(52.0, 16.0, 0), Some((0, "b0")));
        assert_eq!(gui.button_at_row(52.0, 16.0, 1), Some((4, "b4")));
        assert_eq!(gui.button_position("b0", 0, 0, 1), None);
        let mut invalid = value.clone();
        invalid["scroll"]["columns"] = 0.into();
        assert!(Gui::parse(&invalid.to_string()).is_err());
        let mut invalid = value;
        invalid["scroll"]["buttons"][0] = "absent".into();
        assert!(Gui::parse(&invalid.to_string()).is_err());
    }
    #[test]
    fn player_inventory_geometry_preserves_legacy_and_rejects_out_of_bounds() {
        let legacy = Gui::parse(r#"{"width":176,"height":166,"slots":[]}"#).unwrap();
        assert_eq!(legacy.player_inventory_origin(), (8, 84));
        let old_button = Gui::parse(r#"{"width":176,"height":166,"slots":[],"elements":[{"type":"button","id":"b","x":0,"y":0,"width":8,"height":8,"text":"B"}]}"#).unwrap();
        let serialized = serde_json::to_value(old_button).unwrap();
        assert!(serialized["elements"][0].get("enabled").is_none());
        let gui = Gui::parse(
            r#"{"width":200,"height":180,"slots":[],"player_inventory":{"x":38,"y":104}}"#,
        )
        .unwrap();
        assert_eq!(gui.player_inventory_origin(), (38, 104));
        let mut invalid = gui.clone();
        invalid.player_inventory.as_mut().unwrap().x += 1;
        assert!(invalid.validate().is_err());
        let mut invalid = gui;
        invalid.player_inventory.as_mut().unwrap().y += 1;
        assert!(invalid.validate().is_err());
    }
    #[test]
    fn storage_and_machine_definitions_are_bounded() {
        let storage = r##"{"width":176,"height":166,"slots":[{"id":"storage0","x":8,"y":18,"role":"storage"},{"id":"fuel","x":26,"y":18,"role":"input","accepts":["#minecraft:coals"]},{"id":"result","x":134,"y":18,"role":"output"}],"elements":[{"type":"progress","id":"energy","x":80,"y":18,"width":24,"height":8},{"type":"button","id":"toggle","x":80,"y":40,"width":24,"height":12,"text":"Run"}]}"##;
        let gui = Gui::parse(storage).unwrap();
        assert_eq!(gui.slots.len(), 3);
        assert_eq!(
            Gui::parse(&serde_json::to_string(&gui).unwrap()).unwrap(),
            gui
        );
        let mut invalid = gui.clone();
        invalid.slots[1].id = invalid.slots[0].id.clone();
        assert!(invalid.validate().is_err());
        let mut invalid = gui.clone();
        invalid.slots[0].x = 65535;
        assert!(invalid.validate().is_err());
        let mut invalid = gui;
        invalid.slots[2].accepts.push("minecraft:stone".into());
        assert!(invalid.validate().is_err());
    }
}
