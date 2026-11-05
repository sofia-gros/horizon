//! Registry-scoped identities and persistent states for additional liquid families.
//!
//! This is the data layer, not a registration hook in the running game. The
//! vanilla simulation still uses `FluidKind`; integrating scheduling, flow,
//! rendering and bucket transactions must not silently alias these IDs to lava.
use crate::{
    compression::invalid,
    registry::{BlockRegistry, BlockStateId},
};
use horizon_core::{
    position::BlockPos,
    ticks::{SavedTick, TickPriority, TickTypeId},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    io,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_CATALOG: AtomicU64 = AtomicU64::new(1);

/// A runtime identity. Never persist its numeric representation: catalog order
/// can change on resource reload, and identities belong to exactly one catalog.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RegisteredFluidId {
    catalog: u64,
    index: usize,
}

#[derive(Clone, Debug)]
pub struct LiquidFamily {
    pub source: String,
    pub flowing: String,
    pub block: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RegisteredFluidState {
    id: RegisteredFluidId,
    amount: u8,
    falling: bool,
}
impl RegisteredFluidState {
    pub fn id(self) -> RegisteredFluidId {
        self.id
    }
    pub fn amount(self) -> u8 {
        self.amount
    }
    pub fn falling(self) -> bool {
        self.falling
    }
    pub fn is_source(self) -> bool {
        self.id.index % 2 == 0
    }
    pub fn own_height(self) -> f32 {
        f32::from(self.amount) / 9.0
    }
}

/// Save format deliberately uses the full namespaced identity, not a transient
/// numeric ID. Removed/unknown families fail to load rather than becoming lava.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedFluidState {
    pub id: String,
    pub amount: u8,
    pub falling: bool,
}

/// A named-catalog tick before insertion into a runtime scheduler.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RegisteredFluidTick {
    pub id: RegisteredFluidId,
    pub pos: BlockPos,
    pub delay: i32,
    pub priority: TickPriority,
}

struct Family {
    names: [String; 2],
    levels: [BlockStateId; 16],
}

/// All bindings are checked together before publishing a catalog. The borrowed
/// block registry prevents retaining bindings beyond their resource lifetime.
pub struct RegisteredFluids<'a> {
    registry: &'a BlockRegistry,
    identity: u64,
    families: Vec<Family>,
    names: BTreeMap<String, usize>,
    blocks: BTreeMap<BlockStateId, (usize, u8)>,
}
impl<'a> RegisteredFluids<'a> {
    pub fn new(
        registry: &'a BlockRegistry,
        definitions: impl IntoIterator<Item = LiquidFamily>,
    ) -> io::Result<Self> {
        let mut result = Self {
            registry,
            identity: NEXT_CATALOG
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| v.checked_add(1))
                .map_err(|_| invalid("fluid catalog identity exhausted"))?,
            families: Vec::new(),
            names: BTreeMap::new(),
            blocks: BTreeMap::new(),
        };
        for definition in definitions {
            if !valid_name(&definition.source)
                || !valid_name(&definition.flowing)
                || !valid_name(&definition.block)
            {
                return Err(invalid("invalid namespaced liquid family identifier"));
            }
            if definition.source == definition.flowing
                || result.names.contains_key(&definition.source)
                || result.names.contains_key(&definition.flowing)
            {
                return Err(invalid("duplicate fluid identifier"));
            }
            let default = registry
                .default_state(&definition.block)
                .ok_or_else(|| invalid("missing liquid block"))?;
            let mut state = registry
                .state(default)
                .ok_or_else(|| invalid("invalid liquid block default"))?
                .clone();
            // A liquid block has exactly the vanilla 0..15 level property. Do
            // not silently bind one arbitrary variant of a waterlogged/model block.
            if state.properties.len() != 1 || !state.properties.contains_key("level") {
                return Err(invalid("liquid block requires only a level property"));
            }
            let family = result.families.len();
            let mut levels = [default; 16];
            for (level, id) in levels.iter_mut().enumerate() {
                state.properties.insert("level".into(), level.to_string());
                *id = registry
                    .id(&state)
                    .ok_or_else(|| invalid("missing liquid block level"))?;
                if result.blocks.insert(*id, (family, level as u8)).is_some() {
                    return Err(invalid("liquid block bound to multiple families"));
                }
            }
            for (offset, name) in [&definition.source, &definition.flowing]
                .into_iter()
                .enumerate()
            {
                result.names.insert(name.clone(), family * 2 + offset);
            }
            result.families.push(Family {
                names: [definition.source, definition.flowing],
                levels,
            });
        }
        Ok(result)
    }
    pub fn registry(&self) -> &'a BlockRegistry {
        self.registry
    }
    pub fn id(&self, name: &str) -> Option<RegisteredFluidId> {
        self.names.get(name).map(|&index| RegisteredFluidId {
            catalog: self.identity,
            index,
        })
    }
    fn check(&self, id: RegisteredFluidId) -> io::Result<&Family> {
        if id.catalog != self.identity {
            return Err(invalid("fluid identity belongs to a different catalog"));
        }
        self.families
            .get(id.index / 2)
            .ok_or_else(|| invalid("unknown runtime fluid identity"))
    }
    pub fn name(&self, id: RegisteredFluidId) -> io::Result<&str> {
        Ok(&self.check(id)?.names[id.index % 2])
    }
    pub fn same_family(&self, a: RegisteredFluidId, b: RegisteredFluidId) -> io::Result<bool> {
        self.check(a)?;
        self.check(b)?;
        Ok(a.index / 2 == b.index / 2)
    }
    pub fn source(&self, id: RegisteredFluidId) -> io::Result<RegisteredFluidId> {
        self.check(id)?;
        Ok(RegisteredFluidId {
            index: id.index / 2 * 2,
            ..id
        })
    }
    pub fn flowing(&self, id: RegisteredFluidId) -> io::Result<RegisteredFluidId> {
        Ok(RegisteredFluidId {
            index: self.source(id)?.index + 1,
            ..id
        })
    }
    pub fn state(
        &self,
        id: RegisteredFluidId,
        amount: u8,
        falling: bool,
    ) -> io::Result<RegisteredFluidState> {
        self.check(id)?;
        if !(1..=8).contains(&amount) || (id.index % 2 == 0 && amount != 8) {
            return Err(invalid("invalid fluid amount"));
        }
        Ok(RegisteredFluidState {
            id,
            amount,
            falling,
        })
    }
    pub fn from_block(&self, block: BlockStateId) -> Option<RegisteredFluidState> {
        let &(family, level) = self.blocks.get(&block)?;
        Some(RegisteredFluidState {
            id: RegisteredFluidId {
                catalog: self.identity,
                index: family * 2 + usize::from(level != 0),
            },
            amount: if level == 0 || level >= 8 {
                8
            } else {
                8 - level
            },
            falling: level >= 8,
        })
    }
    /// Uses vanilla legacy-block canonicalization: falling levels 8..15 read
    /// as a full falling state, and that state writes canonical block level 8.
    pub fn block(&self, state: RegisteredFluidState) -> io::Result<BlockStateId> {
        let family = self.check(state.id)?;
        let level = if state.is_source() {
            0
        } else {
            8 - state.amount + if state.falling { 8 } else { 0 }
        };
        Ok(family.levels[usize::from(level)])
    }
    pub fn save(&self, state: RegisteredFluidState) -> io::Result<SavedFluidState> {
        Ok(SavedFluidState {
            id: self.name(state.id)?.into(),
            amount: state.amount,
            falling: state.falling,
        })
    }
    /// Reuse the existing named NBT tick format without persisting catalog slots.
    pub fn save_ticks(&self, ticks: &[RegisteredFluidTick]) -> io::Result<crate::nbt::Tag> {
        let ticks = ticks
            .iter()
            .map(|tick| {
                self.check(tick.id)?;
                Ok(SavedTick {
                    kind: TickTypeId(
                        u32::try_from(tick.id.index)
                            .map_err(|_| invalid("fluid tick ID overflow"))?,
                    ),
                    pos: tick.pos,
                    delay: tick.delay,
                    priority: tick.priority,
                })
            })
            .collect::<io::Result<Vec<_>>>()?;
        crate::tick_data::write_list(&ticks, |id| {
            self.families
                .get(id.0 as usize / 2)
                .map(|family| family.names[id.0 as usize % 2].clone())
        })
    }
    pub fn load_ticks(&self, tag: &crate::nbt::Tag) -> io::Result<Vec<RegisteredFluidTick>> {
        crate::tick_data::read_list(tag, |name| {
            self.names
                .get(name)
                .and_then(|&index| u32::try_from(index).ok())
                .map(TickTypeId)
        })?
        .into_iter()
        .map(|tick| {
            Ok(RegisteredFluidTick {
                id: RegisteredFluidId {
                    catalog: self.identity,
                    index: tick.kind.0 as usize,
                },
                pos: tick.pos,
                delay: tick.delay,
                priority: tick.priority,
            })
        })
        .collect()
    }
    pub fn load(&self, saved: &SavedFluidState) -> io::Result<RegisteredFluidState> {
        let id = self
            .id(&saved.id)
            .ok_or_else(|| invalid("saved fluid is not registered"))?;
        self.state(id, saved.amount, saved.falling)
    }
}

fn valid_name(name: &str) -> bool {
    let Some((namespace, path)) = name.split_once(':') else {
        return false;
    };
    let valid =
        |b: u8| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'_' | b'-' | b'.');
    !namespace.is_empty()
        && !path.is_empty()
        && namespace.bytes().all(valid)
        && path.bytes().all(|b| valid(b) || b == b'/')
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn blocks(missing_last: bool) -> BlockRegistry {
        let mut rows = vec![json!(["minecraft:air", true, false, "", "minecraft:air"])];
        let mut defaults = serde_json::Map::new();
        defaults.insert("minecraft:air".into(), json!(0));
        for name in ["test:one", "test:two"] {
            defaults.insert(name.into(), json!(rows.len()));
            for level in 0..if missing_last && name == "test:two" {
                15
            } else {
                16
            } {
                rows.push(json!([name, false, true, "", {"id":name, "properties":{"level":level.to_string()}}]));
            }
        }
        BlockRegistry::from_facts_json(
            json!({"states": rows, "defaults":defaults})
                .to_string()
                .as_bytes(),
        )
        .unwrap()
    }
    fn family(name: &str) -> LiquidFamily {
        LiquidFamily {
            source: format!("test:{name}"),
            flowing: format!("test:flowing_{name}"),
            block: format!("test:{name}"),
        }
    }
    #[test]
    fn distinct_families_and_source_flowing_pairs() {
        let blocks = blocks(false);
        let fluids = RegisteredFluids::new(&blocks, [family("one"), family("two")]).unwrap();
        let a = fluids.id("test:one").unwrap();
        let b = fluids.id("test:two").unwrap();
        assert!(fluids.same_family(a, fluids.flowing(a).unwrap()).unwrap());
        assert!(!fluids.same_family(a, b).unwrap());
        assert_eq!(fluids.source(fluids.flowing(a).unwrap()).unwrap(), a);
        assert!(fluids.id("minecraft:lava").is_none());
        assert!(fluids.from_block(BlockStateId(0)).is_none());
    }
    #[test]
    fn every_block_level_decodes_and_canonicalizes() {
        let blocks = blocks(false);
        let fluids = RegisteredFluids::new(&blocks, [family("one")]).unwrap();
        for level in 0..16 {
            let state = fluids.from_block(BlockStateId(1 + level)).unwrap();
            assert_eq!(state.is_source(), level == 0);
            assert_eq!(state.falling(), level >= 8);
            assert_eq!(
                state.amount(),
                if level == 0 || level >= 8 {
                    8
                } else {
                    8 - level as u8
                }
            );
            assert_eq!(fluids.block(state).unwrap(), BlockStateId(1 + level.min(8)));
        }
    }
    #[test]
    fn save_names_survive_reordered_catalogs_but_runtime_ids_do_not() {
        let blocks = blocks(false);
        let old = RegisteredFluids::new(&blocks, [family("one"), family("two")]).unwrap();
        let new = RegisteredFluids::new(&blocks, [family("two"), family("one")]).unwrap();
        let state = old
            .state(old.id("test:flowing_one").unwrap(), 3, true)
            .unwrap();
        let json = serde_json::to_string(&old.save(state).unwrap()).unwrap();
        let saved: SavedFluidState = serde_json::from_str(&json).unwrap();
        let restored = new.load(&saved).unwrap();
        assert_eq!(new.name(restored.id()).unwrap(), "test:flowing_one");
        assert_eq!((restored.amount(), restored.falling()), (3, true));
        assert!(new.block(state).is_err());
        assert!(new.save(state).is_err());
        assert!(new.same_family(state.id(), restored.id()).is_err());
    }
    #[test]
    fn named_ticks_preserve_metadata_across_reload_and_reject_missing_families() {
        let blocks = blocks(false);
        let old = RegisteredFluids::new(&blocks, [family("one"), family("two")]).unwrap();
        let new = RegisteredFluids::new(&blocks, [family("two"), family("one")]).unwrap();
        let tick = RegisteredFluidTick {
            id: old.id("test:flowing_one").unwrap(),
            pos: BlockPos::new(-16, -64, 300),
            delay: -5,
            priority: TickPriority::Normal,
        };
        let saved = old.save_ticks(&[tick]).unwrap();
        let restored = new.load_ticks(&saved).unwrap();
        assert_eq!(
            restored,
            vec![RegisteredFluidTick {
                id: new.id("test:flowing_one").unwrap(),
                ..tick
            }]
        );
        assert!(new.save_ticks(&[tick]).is_err());
        let removed = RegisteredFluids::new(&blocks, [family("two")]).unwrap();
        assert!(removed.load_ticks(&saved).is_err());
    }
    #[test]
    fn missing_or_invalid_saved_fluids_fail_closed() {
        let blocks = blocks(false);
        let fluids = RegisteredFluids::new(&blocks, [family("one")]).unwrap();
        for (id, amount) in [
            ("test:missing", 8),
            ("test:one", 1),
            ("test:flowing_one", 0),
            ("test:flowing_one", 9),
        ] {
            assert!(
                fluids
                    .load(&SavedFluidState {
                        id: id.into(),
                        amount,
                        falling: false
                    })
                    .is_err()
            );
        }
        assert!(
            serde_json::from_str::<SavedFluidState>(
                r#"{"id":"test:one","amount":8,"falling":false,"runtime_id":1}"#
            )
            .is_err()
        );
    }
    #[test]
    fn duplicate_names_or_blocks_and_missing_levels_are_rejected() {
        let blocks = blocks(false);
        assert!(RegisteredFluids::new(&blocks, [family("one"), family("one")]).is_err());
        let mut shared = family("two");
        shared.block = "test:one".into();
        assert!(RegisteredFluids::new(&blocks, [family("one"), shared]).is_err());
        let missing = blocks_fn_missing();
        assert!(RegisteredFluids::new(&missing, [family("one"), family("two")]).is_err());
        let mut equal = family("one");
        equal.flowing = equal.source.clone();
        assert!(RegisteredFluids::new(&blocks, [equal]).is_err());
    }
    fn blocks_fn_missing() -> BlockRegistry {
        blocks(true)
    }
    #[test]
    fn invalid_names_and_non_liquid_blocks_are_rejected() {
        let blocks = blocks(false);
        for name in [
            "one",
            ":one",
            "test:",
            "Test:one",
            "test:one:two",
            "test:a b",
        ] {
            let mut bad = family("one");
            bad.source = name.into();
            assert!(RegisteredFluids::new(&blocks, [bad]).is_err(), "{name}");
        }
        let mut air = family("one");
        air.block = "minecraft:air".into();
        assert!(RegisteredFluids::new(&blocks, [air]).is_err());
    }
}
