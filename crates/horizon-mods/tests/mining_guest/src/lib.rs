use horizon_mod_sdk::server::{Registration, ServerMod, items_edit, items_tools};

struct MiningGuest;
impl ServerMod for MiningGuest {
    fn init() -> Result<(Self, Registration), String> {
        items_tools::enable_mining_events();
        items_tools::enable_melee_events();
        Ok((Self, Registration::new()))
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
