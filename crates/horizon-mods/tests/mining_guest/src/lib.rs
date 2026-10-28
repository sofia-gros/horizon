use horizon_mod_sdk::server::{
    Args, GuiWorkstationOutput, GuiWorkstationPlan, Player, Registration, ServerMod,
    WorkstationInput, WorkstationPlan, inventory_transactions, items_edit, items_tools, players,
    workstations,
};

struct MiningGuest;
impl ServerMod for MiningGuest {
    fn init() -> Result<(Self, Registration), String> {
        items_tools::enable_mining_events();
        items_tools::enable_melee_events();
        workstations::register(
            "example:station",
            "Fixture",
            &["example".into(), "state".into()],
            &["example:ingredient".into()],
        )?;
        workstations::register_gui(
            "example:gui_station",
            "GUI Fixture",
            &["example".into(), "state".into()],
            &[],
            r#"{"width":176,"height":166,"slots":[{"id":"pattern","x":8,"y":18,"role":"input"},{"id":"material","x":26,"y":18,"role":"input"},{"id":"result","x":134,"y":18,"role":"output"}],"elements":[{"type":"button","id":"choose","x":60,"y":18,"width":24,"height":12,"text":"Pick"},{"type":"progress","id":"work","x":80,"y":40,"width":24,"height":8,"value":10}]}"#,
        )?;
        Ok((
            Self,
            Registration::new()
                .command("online")
                .command("miner-only")
                .command("consume-fixture"),
        ))
    }

    fn gui_workstation_preview(
        &mut self,
        _: &str,
        inputs: &[Option<WorkstationInput>],
        selection: Option<&str>,
    ) -> Result<Option<GuiWorkstationPlan>, String> {
        assert!(horizon_mod_sdk::storage::set("preview-write", b"no").is_err());
        assert!(
            workstations::set_progress(
                "example:gui_station",
                "minecraft:overworld",
                0,
                71,
                2,
                "work",
                99
            )
            .is_err()
        );
        if inputs.len() != 3 || selection != Some("choose") {
            return Ok(None);
        }
        Ok(Some(GuiWorkstationPlan {
            consume: vec![0, 1, 0],
            outputs: vec![GuiWorkstationOutput {
                item: "example:part".into(),
                count: 1,
                source_slot: None,
                text: None,
                tool: None,
                durability: None,
                melee: None,
            }],
        }))
    }

    fn workstation_preview(
        &mut self,
        _: &str,
        tool: &WorkstationInput,
        ingredient: &WorkstationInput,
    ) -> Result<Option<WorkstationPlan>, String> {
        players::broadcast("discard preview side effect");
        if ingredient.item.starts_with("storage-") {
            assert!(horizon_mod_sdk::storage::set("created", b"changed").is_err());
            assert!(horizon_mod_sdk::storage::set("existing", b"changed").is_err());
            horizon_mod_sdk::storage::delete("existing");
            match ingredient.item.as_str() {
                "storage-trap" => panic!("preview trap"),
                "storage-reject" => return Err("preview rejected".into()),
                "storage-none" => return Ok(None),
                _ => {}
            }
        }
        if tool.custom_text.as_deref() != Some("original")
            || !ingredient.tags.iter().any(|t| t == "example:ingredient")
        {
            return Ok(None);
        }
        Ok(Some(WorkstationPlan {
            replacement: "repaired".into(),
            consume: 1,
            tool: items_tools::Profile {
                rules: vec![],
                default_speed: 1.0,
                damage_per_block: 0,
                can_destroy_in_creative: true,
                mining_enabled: true,
                unbreakable: false,
            },
            damage: 0,
            max_damage: 10,
            melee: items_tools::MeleeProfile {
                damage_bonus: 0.0,
                damage_multiplier: 1.0,
                attack_speed: 1.0,
                enabled: true,
            },
        }))
    }

    fn command(
        &mut self,
        player: &Player,
        name: &str,
        args: &Args,
    ) -> Result<Option<String>, String> {
        if name == "online" && args.text() == "storage-write" {
            horizon_mod_sdk::storage::set("normal", b"ok")?;
            workstations::set_progress(
                "example:gui_station",
                "minecraft:overworld",
                0,
                71,
                2,
                "work",
                73,
            )?;
        }
        if name == "consume-fixture" {
            let target = if args.text() == "other" {
                "another-player"
            } else {
                &player.id
            };
            let input =
                inventory_transactions::inventory_slot(target, 1, &[])?.ok_or("no ingredient")?;
            if input.item != "example:ingredient" {
                return Err("wrong ingredient".into());
            }
            inventory_transactions::replace_main_hand_combat_profile_consuming(
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
