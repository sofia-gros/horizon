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
    #[serde(default)]
    pub elements: Vec<Element>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerInventory {
    pub x: u16,
    pub y: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Slot {
    pub id: String,
    pub x: u16,
    pub y: u16,
    pub role: SlotRole,
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
    },
    Image {
        x: u16,
        y: u16,
        width: u16,
        height: u16,
        texture: String,
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
                (enabled
                    && x >= f32::from(bx)
                    && y >= f32::from(by)
                    && x < f32::from(bx) + f32::from(width)
                    && y < f32::from(by) + f32::from(height))
                .then_some((index, id.as_str()))
            })
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
                Element::Label { x, y, text } => {
                    inside(*x, *y, 1, 1) && text.len() <= 256 && !text.chars().any(char::is_control)
                }
                Element::Image {
                    x,
                    y,
                    width,
                    height,
                    texture,
                } => inside(*x, *y, *width, *height) && image_valid(texture),
                Element::Button {
                    id,
                    x,
                    y,
                    width,
                    height,
                    text,
                    images,
                    ..
                } => {
                    id_valid(id)
                        && ids.insert(id)
                        && inside(*x, *y, *width, *height)
                        && text.len() <= 128
                        && !text.chars().any(char::is_control)
                        && images.as_ref().is_none_or(|images| {
                            image_valid(&images.normal)
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
        assert_eq!(gui.button_at(4.0, 4.0), None);
        let mut active = gui.clone();
        if let Element::Button { enabled, .. } = &mut active.elements[1] {
            *enabled = true;
        }
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
