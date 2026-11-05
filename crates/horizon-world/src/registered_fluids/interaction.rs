//! Explicit custom-liquid physics. These parameters never assign a native tag,
//! and absent policies leave contact observations inert.
use crate::{
    compression::invalid,
    living_air::move_relative,
    living_fluid::{LivingFluidWorld, fluid_falling_adjusted_movement},
};
use horizon_core::geometry::Vec3;
use serde::Deserialize;
use std::io;

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TravelPolicy {
    pub acceleration: f32,
    pub horizontal_drag: f64,
    pub vertical_drag: f64,
    pub gravity_scale: f64,
    pub jump: f64,
}
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BurnContact {
    pub seconds: u16,
    pub damage: f32,
}
/// A sustained vanilla effect or a namespaced effect declared by an enabled
/// content mod. Resource consumers must resolve it before enabling the fluid.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub struct ContactEffect(std::sync::Arc<str>);
impl TryFrom<String> for ContactEffect {
    type Error = &'static str;
    fn try_from(id: String) -> Result<Self, Self::Error> {
        if !super::valid_name(&id)
            || (id.starts_with("minecraft:")
                && !matches!(
                    id.as_str(),
                    "minecraft:poison" | "minecraft:slowness" | "minecraft:blindness"
                ))
        {
            return Err("invalid or unsupported registered-fluid effect id");
        }
        Ok(Self(id.into()))
    }
}
impl ContactEffect {
    pub fn id(&self) -> &str {
        &self.0
    }
    pub fn player_id(&self) -> &str {
        self.0.strip_prefix("minecraft:").unwrap_or(self.0.as_ref())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EffectContact {
    pub id: ContactEffect,
    pub duration_ticks: u32,
    pub amplifier: u8,
    /// Explicit empty curative-item policy; omitted keeps ordinary behavior.
    #[serde(default)]
    pub no_curatives: bool,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InteractionPolicy {
    pub travel: TravelPolicy,
    pub motion_scale: f64,
    pub can_swim: bool,
    pub can_drown: bool,
    pub extinguishes: bool,
    pub fall_distance_scale: f64,
    pub burn: Option<BurnContact>,
    pub effect: Option<EffectContact>,
}
impl InteractionPolicy {
    pub fn validate(&self) -> io::Result<()> {
        let t = self.travel;
        if [
            f64::from(t.acceleration),
            t.horizontal_drag,
            t.vertical_drag,
            t.gravity_scale,
            t.jump,
            self.motion_scale,
            self.fall_distance_scale,
        ]
        .into_iter()
        .any(|n| !n.is_finite() || !(0.0..=1.0).contains(&n))
            || self.burn.is_some_and(|b| {
                b.seconds == 0
                    || b.seconds > 3600
                    || !b.damage.is_finite()
                    || !(0.0..=1000.0).contains(&b.damage)
            })
            || self
                .effect
                .as_ref()
                .is_some_and(|e| !(1..=72_000).contains(&e.duration_ticks))
        {
            return Err(invalid("invalid registered fluid interaction policy"));
        }
        Ok(())
    }
}
impl TravelPolicy {
    /// Configured drag and gravity after collision-resolved movement. Keep this
    /// separate from native water efficiency, dolphin effects and lava damage.
    pub fn damp(self, velocity: Vec3, gravity: f64) -> Vec3 {
        Vec3::new(
            velocity.x * self.horizontal_drag,
            velocity.y * self.vertical_drag - gravity * self.gravity_scale,
            velocity.z * self.horizontal_drag,
        )
    }
    pub fn travel<W: LivingFluidWorld>(self, world: &mut W, input: Vec3) -> Result<(), W::Error> {
        let gravity = world.effective_gravity()?;
        let falling = world.velocity().y <= 0.0;
        move_relative(world, input, self.acceleration)?;
        world.move_self(world.velocity())?;
        let velocity = self.damp(world.velocity(), 0.0);
        let velocity = fluid_falling_adjusted_movement(
            world,
            gravity * self.gravity_scale * 16.0,
            falling,
            velocity,
        )?;
        world.set_velocity(velocity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn policy() -> InteractionPolicy {
        InteractionPolicy {
            travel: TravelPolicy {
                acceleration: 0.02,
                horizontal_drag: 0.8,
                vertical_drag: 0.8,
                gravity_scale: 0.0625,
                jump: 0.04,
            },
            motion_scale: 0.0023333333333333335,
            can_swim: false,
            can_drown: false,
            extinguishes: false,
            fall_distance_scale: 0.5,
            burn: Some(BurnContact {
                seconds: 10,
                damage: 5.0,
            }),
            effect: None,
        }
    }
    #[test]
    fn declared_drag_gravity_and_bounds_are_not_native_lava_defaults() {
        let p = policy();
        p.validate().unwrap();
        let v = p.travel.damp(Vec3::new(0.25, -0.1, -0.25), 0.08);
        assert!((v.x - 0.2).abs() < 1e-12);
        assert!((v.y + 0.085).abs() < 1e-12);
        assert!((v.z + 0.2).abs() < 1e-12);
        let mut bad = p.clone();
        bad.motion_scale = f64::NAN;
        assert!(bad.validate().is_err());
        bad = p.clone();
        bad.travel.horizontal_drag = 1.01;
        assert!(bad.validate().is_err());
        bad = p;
        bad.burn.as_mut().unwrap().damage = -1.0;
        assert!(bad.validate().is_err());
    }
}
