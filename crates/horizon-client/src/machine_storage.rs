//! Atomic planning for persistent mod machine storage.
use crate::items::ItemStack;
use horizon_mods::{MachineChange, gui::Slot};

/// Build a new inventory without mutating the captured/current one. The caller
/// must first compare its entire snapshot and verify the block and ownership.
pub fn plan(
    current: &[Option<ItemStack>],
    expected: &[Option<ItemStack>],
    slots: &[&Slot],
    changes: &[MachineChange],
    max_stack: impl Fn(&str) -> Option<u32>,
    in_tag: impl Fn(&str, &str) -> bool,
) -> Result<Vec<Option<ItemStack>>, &'static str> {
    if current != expected {
        return Err("storage changed after snapshot");
    }
    if current.len() != slots.len() || changes.is_empty() || changes.len() > slots.len() {
        return Err("invalid storage layout or changes");
    }
    let accepts = |slot: &Slot, item: &str| {
        slot.accepts.is_empty()
            || slot.accepts.iter().any(|selector| {
                selector
                    .strip_prefix('#')
                    .map_or_else(|| selector == item, |tag| in_tag(tag, item))
            })
    };
    let mut result = current.to_vec();
    let mut seen = std::collections::HashSet::new();
    for change in changes {
        if !seen.insert(&change.slot)
            || change.item.is_some() != (change.count > 0)
            || (change.consume == 0 && change.count == 0)
        {
            return Err("invalid storage change");
        }
        let index = slots
            .iter()
            .position(|s| s.id == change.slot)
            .ok_or("unknown storage slot")?;
        let cell = &mut result[index];
        if cell
            .as_ref()
            .is_some_and(|stack| !accepts(slots[index], &stack.item))
        {
            return Err("slot rejects existing input");
        }
        if change.consume > cell.as_ref().map_or(0, |s| s.count) {
            return Err("insufficient input");
        }
        if change.consume > 0 {
            let stack = cell.as_mut().ok_or("missing input")?;
            stack.count -= change.consume;
            if stack.count == 0 {
                *cell = None;
            }
        }
        if let Some(item) = &change.item {
            let max = max_stack(item).ok_or("unknown output item")?.min(99);
            if !accepts(slots[index], item) {
                return Err("slot rejects output");
            }
            if change.count == 0 || change.count > max {
                return Err("invalid output count");
            }
            let output = ItemStack::new(item, change.count);
            if let Some(stack) = cell {
                let total = stack
                    .count
                    .checked_add(change.count)
                    .ok_or("output does not fit")?;
                if !stack.same_item_same_components(&output) || total > max {
                    return Err("output does not fit");
                }
                stack.count = total;
            } else {
                *cell = Some(output);
            }
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_mods::gui::SlotRole;
    use horizon_world::nbt::Tag;
    fn slot(id: &str, item: &str) -> Slot {
        Slot {
            id: id.into(),
            x: 0,
            y: 0,
            role: SlotRole::Storage,
            accepts: vec![item.into()],
        }
    }
    fn change(slot: &str, consume: u32, item: Option<&str>, count: u32) -> MachineChange {
        MachineChange {
            slot: slot.into(),
            consume,
            item: item.map(str::to_owned),
            count,
        }
    }
    #[test]
    fn consumes_multiple_inputs_preserving_foreign_data_and_rejects_full_output_atomically() {
        let specs = [
            slot("a", "minecraft:flint"),
            slot("b", "minecraft:coal"),
            slot("c", "minecraft:diamond"),
        ];
        let refs: Vec<_> = specs.iter().collect();
        let mut flint = ItemStack::new("minecraft:flint", 2);
        flint.data_mut().set_other(
            "minecraft:custom_data",
            Some(Tag::compound([(
                "other_mod",
                Tag::compound([("marker", Tag::Int(7))]),
            )])),
        );
        let initial = vec![
            Some(flint.clone()),
            Some(ItemStack::new("minecraft:coal", 1)),
            Some(ItemStack::new("minecraft:diamond", 64)),
        ];
        let changes = vec![
            change("a", 1, None, 0),
            change("b", 1, None, 0),
            change("c", 0, Some("minecraft:diamond"), 1),
        ];
        let max = |_: &str| Some(64);
        assert!(plan(&initial, &initial, &refs, &changes, max, |_, _| false).is_err());
        assert_eq!(initial[0], Some(flint.clone()));
        let mut available = initial.clone();
        available[2] = None;
        let result = plan(&available, &available, &refs, &changes, max, |_, _| false).unwrap();
        let mut expected = flint;
        expected.count = 1;
        assert_eq!(result[0], Some(expected));
        assert_eq!(result[1], None);
        assert_eq!(result[2], Some(ItemStack::new("minecraft:diamond", 1)));
        assert!(plan(&available, &initial, &refs, &changes, max, |_, _| false).is_err());
        let mut invalid_input = available.clone();
        invalid_input[0] = Some(ItemStack::new("minecraft:coal", 2));
        assert!(
            plan(
                &invalid_input,
                &invalid_input,
                &refs,
                &changes,
                max,
                |_, _| false
            )
            .is_err()
        );
        let wrong = [change("c", 0, Some("minecraft:coal"), 1)];
        assert!(plan(&available, &available, &refs, &wrong, max, |_, _| false).is_err());
        assert!(
            plan(
                &available,
                &available,
                &refs,
                &[changes[0].clone(), changes[0].clone()],
                max,
                |_, _| false
            )
            .is_err()
        );
    }
}
