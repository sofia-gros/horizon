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
    #[serde(default)]
    pub elements: Vec<Element>,
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
    Button {
        id: String,
        x: u16,
        y: u16,
        width: u16,
        height: u16,
        text: String,
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
                Element::Button {
                    id,
                    x,
                    y,
                    width,
                    height,
                    text,
                } => {
                    id_valid(id)
                        && ids.insert(id)
                        && inside(*x, *y, *width, *height)
                        && text.len() <= 128
                        && !text.chars().any(char::is_control)
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
