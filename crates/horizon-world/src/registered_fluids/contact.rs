//! Bounded geometric contact sampling; identities remain separate from native
//! water/lava physics. Effects and travel policies consume these observations.
use super::{RegisteredFluidId, RegisteredFluids};
use crate::{compression::invalid, entity_fluid::CurrentAccumulator, registry::BlockStateId};
use horizon_core::{
    geometry::{Aabb, Vec3},
    position::BlockPos,
};
use std::io;

#[derive(Clone, Copy, Debug)]
pub struct Contact {
    pub source: RegisteredFluidId,
    /// Highest intersecting surface above the entity's bounding-box minimum.
    pub height: f64,
    pub eyes_inside: bool,
    pub current: CurrentAccumulator,
}
impl RegisteredFluids<'_> {
    /// `block` returns None for unloaded/out-of-height cells and must never load
    /// chunks. Reject oversized/nonfinite queries before touching the provider.
    pub fn contacts(
        &self,
        bounds: Aabb,
        eyes: Vec3,
        budget: usize,
        block: impl FnMut(BlockPos) -> Option<BlockStateId>,
    ) -> io::Result<Vec<Contact>> {
        self.sample_contacts(bounds, eyes, budget, 2, (block, |_| Ok(Vec3::ZERO)))
    }
    /// The flow provider is called only for intersecting body cells. It must
    /// obey the same loaded-only rule; at most one bounded current query per cell.
    pub fn contacts_with_flow(
        &self,
        bounds: Aabb,
        eyes: Vec3,
        budget: usize,
        block: impl FnMut(BlockPos) -> Option<BlockStateId>,
        flow: impl FnMut(BlockPos) -> io::Result<Vec3>,
    ) -> io::Result<Vec<Contact>> {
        // A current samples at most 29 block/fluid neighbors. Reserve 32 reads
        // per cell, including the two geometric surface samples.
        self.sample_contacts(bounds, eyes, budget, 32, (block, flow))
    }
    fn sample_contacts(
        &self,
        bounds: Aabb,
        eyes: Vec3,
        budget: usize,
        reads_per_cell: u64,
        (mut block, mut flow): (
            impl FnMut(BlockPos) -> Option<BlockStateId>,
            impl FnMut(BlockPos) -> io::Result<Vec3>,
        ),
    ) -> io::Result<Vec<Contact>> {
        let lo = bounds.min_position();
        let hi = bounds.max_position();
        if [lo.x, lo.y, lo.z, hi.x, hi.y, hi.z, eyes.x, eyes.y, eyes.z]
            .into_iter()
            .any(|n| !n.is_finite() || n < f64::from(i32::MIN) || n >= f64::from(i32::MAX))
        {
            return Err(invalid(
                "registered contact bounds outside finite block coordinates",
            ));
        }
        if lo.x > hi.x || lo.y > hi.y || lo.z > hi.z {
            return Err(invalid("inverted registered contact bounds"));
        }
        if lo.x == hi.x || lo.y == hi.y || lo.z == hi.z {
            return Ok(Vec::new());
        }
        let low = BlockPos::new(
            lo.x.floor() as i32,
            lo.y.floor() as i32,
            lo.z.floor() as i32,
        );
        let high = BlockPos::new(
            hi.x.ceil() as i32 - 1,
            hi.y.ceil() as i32 - 1,
            hi.z.ceil() as i32 - 1,
        );
        let count = [
            i64::from(high.x) - i64::from(low.x) + 1,
            i64::from(high.y) - i64::from(low.y) + 1,
            i64::from(high.z) - i64::from(low.z) + 1,
        ]
        .into_iter()
        .try_fold(1u64, |a, b| a.checked_mul(b as u64));
        if count
            .and_then(|n| n.checked_mul(reads_per_cell))
            .and_then(|n| n.checked_add(2))
            .is_none_or(|n| n > budget as u64)
        {
            return Err(invalid("registered contact query exceeds budget"));
        }
        let mut contacts: Vec<Contact> = Vec::new();
        let mut surface = |pos: BlockPos| -> io::Result<Option<(RegisteredFluidId, f64)>> {
            let Some(fluid) = block(pos).and_then(|s| self.from_block(s)) else {
                return Ok(None);
            };
            let above =
                block(BlockPos::new(pos.x, pos.y + 1, pos.z)).and_then(|s| self.from_block(s));
            let full = above.is_some_and(|f| self.same_family(fluid.id(), f.id()).unwrap_or(false));
            let height = if full {
                1.0
            } else {
                f64::from(fluid.own_height())
            };
            Ok(Some((self.source(fluid.id())?, f64::from(pos.y) + height)))
        };
        for x in low.x..=high.x {
            for y in low.y..=high.y {
                for z in low.z..=high.z {
                    if let Some((source, top)) = surface(BlockPos::new(x, y, z))? {
                        if top <= lo.y {
                            continue;
                        }
                        let height = top - lo.y;
                        if let Some(contact) = contacts.iter_mut().find(|c| c.source == source) {
                            contact.height = contact.height.max(height);
                        } else {
                            contacts.push(Contact {
                                source,
                                height,
                                eyes_inside: false,
                                current: CurrentAccumulator::default(),
                            });
                        }
                        let contact = contacts.iter_mut().find(|c| c.source == source).unwrap();
                        let mut vector = flow(BlockPos::new(x, y, z))?;
                        if ![vector.x, vector.y, vector.z]
                            .into_iter()
                            .all(f64::is_finite)
                        {
                            return Err(invalid("nonfinite registered fluid current"));
                        }
                        contact.current.height = contact.height;
                        if contact.height < 0.4 {
                            vector = vector.scale(contact.height);
                        }
                        contact.current.accumulate(vector);
                    }
                }
            }
        }
        let eye_cell = BlockPos::new(
            eyes.x.floor() as i32,
            eyes.y.floor() as i32,
            eyes.z.floor() as i32,
        );
        if let Some((source, top)) = surface(eye_cell)? {
            if eyes.y < top {
                if let Some(contact) = contacts.iter_mut().find(|c| c.source == source) {
                    contact.eyes_inside = true;
                } else {
                    contacts.push(Contact {
                        source,
                        height: 0.0,
                        eyes_inside: true,
                        current: CurrentAccumulator::default(),
                    });
                }
            }
        }
        Ok(contacts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{registered_fluids::LiquidFamily, registry::BlockRegistry};
    use serde_json::json;
    #[test]
    fn contacts_distinguish_families_heights_eyes_and_query_limits() {
        let mut rows = Vec::new();
        for name in ["test:a", "test:b"] {
            for level in 0..16 {
                rows.push(json!([name,false,true,"",{"id":name,"properties":{"level":level.to_string()}}]));
            }
        }
        let registry = BlockRegistry::from_facts_json(
            json!({"states":rows,"defaults":{"test:a":0,"test:b":16}})
                .to_string()
                .as_bytes(),
        )
        .unwrap();
        let fluids = RegisteredFluids::new(
            &registry,
            ["a", "b"].map(|n| LiquidFamily {
                source: format!("test:{n}"),
                flowing: format!("test:flowing_{n}"),
                block: format!("test:{n}"),
            }),
        )
        .unwrap();
        let bounds = Aabb::new(Vec3::new(0.1, 0.0, 0.1), Vec3::new(1.9, 1.8, 0.9));
        let contacts = fluids
            .contacts(bounds, Vec3::new(0.5, 0.8, 0.5), 100, |p| {
                match (p.x, p.y, p.z) {
                    (0, 0, 0) => Some(BlockStateId(0)),
                    (1, 0, 0) => Some(BlockStateId(20)),
                    _ => None,
                }
            })
            .unwrap();
        assert_eq!(contacts.len(), 2);
        assert_eq!(contacts[0].source, fluids.id("test:a").unwrap());
        assert!(contacts[0].eyes_inside);
        assert!(!contacts[1].eyes_inside);
        assert_eq!(contacts[0].height, f64::from(8.0f32 / 9.0));
        assert_eq!(contacts[1].height, f64::from(4.0f32 / 9.0));
        let currents = fluids
            .contacts_with_flow(
                bounds,
                Vec3::new(0.5, 0.8, 0.5),
                1024,
                |p| match (p.x, p.y, p.z) {
                    (0, 0, 0) => Some(BlockStateId(0)),
                    (1, 0, 0) => Some(BlockStateId(20)),
                    _ => None,
                },
                |p| Ok(Vec3::new(if p.x == 0 { 1.0 } else { -1.0 }, 0.0, 0.0)),
            )
            .unwrap();
        assert_eq!(currents[0].current.accumulated.x, 1.0);
        assert_eq!(currents[1].current.accumulated.x, -1.0);
        assert_eq!(currents[0].current.count, 1);
        assert!(
            fluids
                .contacts_with_flow(
                    bounds,
                    Vec3::ZERO,
                    10,
                    |_| panic!("reserve flow budget before reads"),
                    |_| panic!("reserve before currents")
                )
                .is_err()
        );
        let full = fluids
            .contacts(bounds, Vec3::new(0.5, 0.95, 0.5), 100, |p| {
                ((p.x, p.z) == (0, 0) && (0..=1).contains(&p.y)).then_some(BlockStateId(0))
            })
            .unwrap();
        assert!(full[0].eyes_inside);
        assert!(
            fluids
                .contacts(bounds, Vec3::ZERO, 1, |_| panic!(
                    "budget must fail before reads"
                ))
                .is_err()
        );
        let invalid = Aabb::new(Vec3::new(f64::NAN, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0));
        assert!(
            fluids
                .contacts(invalid, Vec3::ZERO, 100, |_| panic!(
                    "invalid bounds must fail before reads"
                ))
                .is_err()
        );
        assert!(
            fluids
                .contacts(bounds, Vec3::ZERO, 100, |_| None)
                .unwrap()
                .is_empty()
        );
    }
}
