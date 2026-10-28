use super::{State, WorkstationRegistration};

impl super::server_bindings::horizon::mods::workstations::Host for State {
    fn register(
        &mut self,
        block: String,
        title: String,
        path: Vec<String>,
        ingredient_tags: Vec<String>,
    ) -> Result<(), String> {
        let valid_id = |id: &str| {
            id.split_once(':').is_some_and(|(ns, name)| {
                !ns.is_empty()
                    && !name.is_empty()
                    && ns.bytes().all(|b| {
                        b.is_ascii_lowercase() || b.is_ascii_digit() || b"_.-".contains(&b)
                    })
                    && name.bytes().all(|b| {
                        b.is_ascii_lowercase() || b.is_ascii_digit() || b"_./-".contains(&b)
                    })
            })
        };
        if !self.registering_workstations
            || self.workstations.len() >= 32
            || block.len() > 128
            || !valid_id(&block)
            || title.is_empty()
            || title.len() > 128
            || title.chars().any(char::is_control)
            || path.is_empty()
            || path.len() > 8
            || path.iter().any(|key| key.is_empty() || key.len() > 128)
            || path[0] == "horizon:tool_control"
            || ingredient_tags.len() > 16
            || ingredient_tags
                .iter()
                .any(|tag| tag.len() > 128 || !valid_id(tag))
            || self.workstations.iter().any(|w| w.block == block)
        {
            return Err("invalid or duplicate workstation registration".into());
        }
        self.workstations.push(WorkstationRegistration {
            owner: self.id.clone(),
            block,
            title,
            path,
            ingredient_tags,
            gui: None,
        });
        Ok(())
    }

    fn register_gui(
        &mut self,
        block: String,
        title: String,
        path: Vec<String>,
        tags: Vec<String>,
        definition: String,
    ) -> Result<(), String> {
        let gui = crate::gui::Gui::parse(&definition)?;
        self.register(block, title, path, tags)?;
        self.workstations.last_mut().expect("registered").gui = Some(gui);
        Ok(())
    }

    fn set_progress(
        &mut self,
        block: String,
        dimension: String,
        x: i32,
        y: i32,
        z: i32,
        id: String,
        value: u8,
    ) -> Result<(), String> {
        if self.previewing_workstation {
            return Err("progress updates are forbidden during workstation preview".into());
        }
        if value > 100 || dimension.is_empty() || dimension.len() > 128 {
            return Err("invalid progress value or dimension".into());
        }
        let registered = self.workstations.iter().find(|r| r.block == block).and_then(|r| r.gui.as_ref()).is_some_and(|gui| gui.elements.iter().any(|element| matches!(element, crate::gui::Element::Progress { id: progress, .. } if progress == &id)));
        if !registered {
            return Err("unknown workstation progress element".into());
        }
        let key = (block, dimension, x, y, z, id);
        self.gui_progress.set(key, value)
    }
}
