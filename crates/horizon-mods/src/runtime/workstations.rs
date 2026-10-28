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
        });
        Ok(())
    }
}
