use super::{ServerHeldStack, State, server_bindings::horizon::mods::items_read};

pub const ITEM_TEXT_MAX: usize = 4096;
pub(super) const READS_PER_CALL: usize = 16;

/// The game lends immutable, owned stack snapshots. Implementations must bound
/// string conversion before allocating; the runtime checks the result as well.
pub trait ItemReader: std::fmt::Debug + Send + Sync {
    fn main_hand(&self, player: &str, path: &[String]) -> Result<Option<ServerHeldStack>, String>;
}

fn read(
    reader: Option<&dyn ItemReader>,
    remaining: &mut usize,
    player: &str,
    path: &[String],
) -> Result<Option<ServerHeldStack>, String> {
    if *remaining == 0 {
        return Err("items-read budget exhausted".into());
    }
    *remaining -= 1;
    if player.len() > 64
        || path.len() > 8
        || path.iter().any(|key| key.is_empty() || key.len() > 128)
    {
        return Err("invalid items-read player or custom path".into());
    }
    let item = reader.map_or(Ok(None), |reader| reader.main_hand(player, path))?;
    if item.as_ref().is_some_and(|item| {
        item.item.len() > 256
            || item
                .custom_text
                .as_ref()
                .is_some_and(|text| text.len() > ITEM_TEXT_MAX)
    }) {
        return Err("items-read result exceeds its size limit".into());
    }
    Ok(item)
}

impl items_read::Host for State {
    fn main_hand(
        &mut self,
        player: String,
        custom_path: Vec<String>,
    ) -> Result<Option<ServerHeldStack>, String> {
        read(
            self.snapshot.items.as_deref(),
            &mut self.item_reads_left,
            &player,
            &custom_path,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct Oversized;
    impl ItemReader for Oversized {
        fn main_hand(&self, _: &str, _: &[String]) -> Result<Option<ServerHeldStack>, String> {
            Ok(Some(ServerHeldStack {
                item: "example:tool".into(),
                count: 1,
                damage: 0,
                custom_text: Some("x".repeat(ITEM_TEXT_MAX + 1)),
            }))
        }
    }

    #[test]
    fn reads_are_bounded_and_never_truncate_a_state() {
        let mut budget = READS_PER_CALL;
        assert!(
            read(Some(&Oversized), &mut budget, "player", &[])
                .unwrap_err()
                .contains("size limit")
        );
        assert!(read(None, &mut budget, "player", &[String::new()]).is_err());
        assert!(read(None, &mut budget, "player", &vec!["x".into(); 9]).is_err());
        assert!(read(None, &mut budget, "player", &["x".repeat(129)]).is_err());
        while budget > 0 {
            assert!(read(None, &mut budget, "offline", &[]).unwrap().is_none());
        }
        assert!(
            read(None, &mut budget, "offline", &[])
                .unwrap_err()
                .contains("budget")
        );
    }
}
