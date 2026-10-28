//! Compare-and-apply primitive for a single inventory's item stacks.
//!
//! The caller owns the inventory, serializes access and must include every
//! recipe input and output slot in the read set, even when a slot is unchanged.
//! This primitive does not authorize recipes, items, players or UI actions.
use crate::items::ItemStack;
use std::collections::BTreeSet;

/// A checked slot replacement. Set `replacement` equal to `expected` for a
/// read-only precondition such as a reusable pattern.
#[derive(Clone, Debug, PartialEq)]
pub struct SlotChange {
    pub slot: usize,
    pub expected: Option<ItemStack>,
    pub replacement: Option<ItemStack>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransactionError {
    Empty,
    TooManySlots,
    StaleRevision,
    RevisionExhausted,
    InvalidSlot,
    DuplicateSlot,
    ChangedSlot,
    InvalidOutput,
}

/// Check the revision, complete stack values, distinct indices and replacement
/// policy before writing any slot. The policy must reject unknown items and
/// stacks exceeding their actual item/container limits. On any error both the
/// inventory and revision remain unchanged. Every inventory writer must advance
/// the same revision, and the caller must keep the whole operation serialized.
pub fn apply(
    slots: &mut [Option<ItemStack>],
    revision: &mut u64,
    expected_revision: u64,
    changes: &[SlotChange],
    valid_output: impl Fn(&ItemStack) -> bool,
) -> Result<(), TransactionError> {
    if changes.is_empty() {
        return Err(TransactionError::Empty);
    }
    if changes.len() > 64 {
        return Err(TransactionError::TooManySlots);
    }
    if *revision != expected_revision {
        return Err(TransactionError::StaleRevision);
    }
    let next = revision
        .checked_add(1)
        .ok_or(TransactionError::RevisionExhausted)?;
    let mut seen = BTreeSet::new();
    for change in changes {
        let actual = slots
            .get(change.slot)
            .ok_or(TransactionError::InvalidSlot)?;
        if !seen.insert(change.slot) {
            return Err(TransactionError::DuplicateSlot);
        }
        if actual != &change.expected {
            return Err(TransactionError::ChangedSlot);
        }
        if change
            .replacement
            .as_ref()
            .is_some_and(|s| s.count == 0 || !valid_output(s))
        {
            return Err(TransactionError::InvalidOutput);
        }
    }
    for change in changes {
        slots[change.slot] = change.replacement.clone();
    }
    *revision = next;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_world::nbt::Tag;

    fn stack(name: &str, count: u32) -> Option<ItemStack> {
        Some(ItemStack::new(name, count))
    }
    fn valid(s: &ItemStack) -> bool {
        matches!(&*s.item, "example:input" | "example:output") && s.count <= 64
    }

    #[test]
    fn competing_transactions_commit_at_most_once() {
        let mut slots = vec![stack("example:input", 2), None];
        let changes = vec![
            SlotChange {
                slot: 0,
                expected: slots[0].clone(),
                replacement: None,
            },
            SlotChange {
                slot: 1,
                expected: None,
                replacement: stack("example:output", 1),
            },
        ];
        let mut revision = 7;
        apply(&mut slots, &mut revision, 7, &changes, valid).unwrap();
        assert_eq!(
            (revision, slots.clone()),
            (8, vec![None, stack("example:output", 1)])
        );
        assert_eq!(
            apply(&mut slots, &mut revision, 7, &changes, valid),
            Err(TransactionError::StaleRevision)
        );
        assert_eq!(
            (revision, slots),
            (8, vec![None, stack("example:output", 1)])
        );
    }

    #[test]
    fn stale_components_or_invalid_output_never_consume_inputs() {
        let mut input = ItemStack::new("example:input", 2);
        input
            .data_mut()
            .set_other("example:state", Some(Tag::Int(1)));
        let mut slots = vec![Some(input.clone()), None];
        let original = slots.clone();
        let mut revision = 0;
        let mut changed = input;
        changed
            .data_mut()
            .set_other("example:state", Some(Tag::Int(2)));
        let mut changes = vec![
            SlotChange {
                slot: 0,
                expected: Some(changed),
                replacement: None,
            },
            SlotChange {
                slot: 1,
                expected: None,
                replacement: stack("example:output", 1),
            },
        ];
        assert_eq!(
            apply(&mut slots, &mut revision, 0, &changes, valid),
            Err(TransactionError::ChangedSlot)
        );
        changes[0].expected = original[0].clone();
        changes[1].replacement = stack("example:unknown", 1);
        assert_eq!(
            apply(&mut slots, &mut revision, 0, &changes, valid),
            Err(TransactionError::InvalidOutput)
        );
        assert_eq!((revision, slots), (0, original));
    }

    #[test]
    fn duplicate_or_out_of_bounds_slots_do_not_write() {
        let mut slots = vec![stack("example:input", 1)];
        let original = slots.clone();
        let mut revision = 0;
        let change = SlotChange {
            slot: 0,
            expected: slots[0].clone(),
            replacement: None,
        };
        assert_eq!(
            apply(
                &mut slots,
                &mut revision,
                0,
                &[change.clone(), change.clone()],
                valid
            ),
            Err(TransactionError::DuplicateSlot)
        );
        let invalid = SlotChange {
            slot: 4,
            expected: None,
            replacement: None,
        };
        assert_eq!(
            apply(&mut slots, &mut revision, 0, &[change, invalid], valid),
            Err(TransactionError::InvalidSlot)
        );
        assert_eq!((revision, slots), (0, original));
    }
}
