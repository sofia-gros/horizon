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

    fn storage_snapshot(
        &mut self,
        block: String,
        dimension: String,
        x: i32,
        y: i32,
        z: i32,
    ) -> Result<super::server_bindings::horizon::mods::workstations::StorageView, String> {
        use super::server_bindings::horizon::mods::workstations::{StorageSlot, StorageView};
        if self.previewing_workstation
            || self.machine_reads.len() >= 16
            || block.len() > 128
            || dimension.is_empty()
            || dimension.len() > 128
        {
            return Err(
                "storage snapshots unavailable during preview or read limit reached".into(),
            );
        }
        let registration = self
            .workstations
            .iter()
            .find(|r| r.block == block && r.gui.is_some())
            .cloned()
            .ok_or("workstation not registered by this mod")?;
        let guard = self.world.lock().map_err(|_| "world unavailable")?;
        let world = guard.as_ref().ok_or("world unavailable")?;
        let pos = [x, y, z];
        let view = world.machine_snapshot(&registration, &dimension, pos)?;
        let slots = view
            .slots
            .iter()
            .map(|slot| StorageSlot {
                id: slot.id.clone(),
                item: slot.item.clone(),
                count: slot.count,
                custom_text: slot.custom_text.clone(),
            })
            .collect();
        drop(guard);
        self.machine_token = self
            .machine_token
            .checked_add(1)
            .ok_or("snapshot token exhausted")?;
        let token = self.machine_token;
        self.machine_reads.push((
            token,
            super::MachineRead {
                registration,
                dimension,
                pos,
                view,
            },
        ));
        Ok(StorageView { token, slots })
    }

    fn storage_commit(
        &mut self,
        token: u64,
        changes: Vec<super::server_bindings::horizon::mods::workstations::StorageChange>,
    ) -> Result<(), String> {
        if self.previewing_workstation || changes.is_empty() || changes.len() > 50 {
            return Err("invalid storage transaction".into());
        }
        let read = self
            .machine_reads
            .iter()
            .find(|(id, _)| *id == token)
            .map(|(_, read)| read)
            .ok_or("invalid snapshot")?
            .clone();
        let mut seen = std::collections::HashSet::new();
        let mut requested = Vec::with_capacity(changes.len());
        for change in changes {
            if change.slot.len() > 128
                || change.item.as_ref().is_some_and(|item| item.len() > 128)
                || change.count > 99
                || !seen.insert(change.slot.clone())
                || !read.view.slots.iter().any(|s| s.id == change.slot)
                || change.item.is_some() != (change.count > 0)
                || (change.consume == 0 && change.count == 0)
            {
                return Err("invalid storage change".into());
            }
            requested.push(super::MachineChange {
                slot: change.slot,
                consume: change.consume,
                item: change.item,
                count: change.count,
            });
        }
        if self.out.iter().any(|o| matches!(o, super::Output::MachineTransaction(t) if std::sync::Arc::ptr_eq(&t.capture, &read.view.capture))) {
            return Err("snapshot already committed".into());
        }
        self.out.push(super::Output::MachineTransaction(
            super::MachineTransaction {
                registration: read.registration,
                dimension: read.dimension,
                pos: read.pos,
                capture: read.view.capture,
                changes: requested,
            },
        ));
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
