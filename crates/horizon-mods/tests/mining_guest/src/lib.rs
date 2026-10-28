use horizon_mod_sdk::server::{
    Args, Player, Registration, ServerMod, items_edit, items_read, items_tools,
};

struct MiningGuest;
impl ServerMod for MiningGuest {
    fn init() -> Result<(Self, Registration), String> {
        items_tools::enable_mining_events();
        items_tools::enable_melee_events();
        Ok((
            Self,
            Registration::new()
                .command("online")
                .command("miner-only")
                .command("consume-fixture"),
        ))
    }

    fn command(
        &mut self,
        player: &Player,
        name: &str,
        args: &Args,
    ) -> Result<Option<String>, String> {
        if name == "consume-fixture" {
            let target = if args.text() == "other" {
                "another-player"
            } else {
                &player.id
            };
            let input = items_read::inventory_slot(target, 1, &[])?.ok_or("no ingredient")?;
            if input.item != "example:ingredient" {
                return Err("wrong ingredient".into());
            }
            items_tools::replace_main_hand_combat_profile_consuming(
                target,
                &["example".into(), "state".into()],
                "original",
                "repaired",
                &items_tools::Profile {
                    rules: vec![],
                    default_speed: 1.0,
                    damage_per_block: 0,
                    can_destroy_in_creative: true,
                    mining_enabled: true,
                    unbreakable: false,
                },
                0,
                10,
                items_tools::MeleeProfile {
                    damage_bonus: 0.0,
                    damage_multiplier: 1.0,
                    attack_speed: 1.0,
                    enabled: true,
                },
                1,
                1,
            )?;
            if args.text() == "error" {
                return Err("fixture rollback".into());
            }
        }
        Ok(Some("mining fixture".into()))
    }

    fn melee_hit(
        &mut self,
        player: &str,
        _: &str,
        _: u64,
        target: &str,
        _: f32,
    ) -> Result<(), String> {
        items_edit::replace_main_hand_text(
            player,
            &["example".into(), "state".into()],
            "original",
            "hit",
        )?;
        if target == "example:error" {
            return Err("fixture error".into());
        }
        if target == "example:trap" {
            panic!("fixture trap");
        }
        Ok(())
    }

    fn mined(
        &mut self,
        player: &str,
        _: &str,
        _: [i32; 3],
        state: &str,
        _: f32,
    ) -> Result<(), String> {
        items_edit::replace_main_hand_text(
            player,
            &["example".into(), "state".into()],
            "original",
            "mined",
        )?;
        if state == "example:trap" {
            panic!("fixture trap");
        }
        Ok(())
    }
}
horizon_mod_sdk::export_server_mod!(MiningGuest);
