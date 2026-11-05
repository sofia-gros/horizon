// Adapted from Tinkers' Construct SlimeBounceHandler,
// Copyright (c) 2022 SlimeKnights, MIT License.
// Full notice in THIRD_PARTY_NOTICES.md. Changed: pure player momentum state,
// finite-input and once-per-host-tick guards; landing restitution is separate.
//! Airborne continuation after an explicitly enabled effect rebound.
use horizon_core::geometry::{Vec3, trig::atan2};

#[derive(Clone, Debug)]
pub struct Momentum {
    last_mag_sq: f64,
    stop_mag_tick: i32,
    was_in_air: bool,
    end_handler: i32,
    last_tick: Option<i32>,
    last_angle: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Update {
    pub active: bool,
    pub velocity: Option<Vec3>,
    pub turn_sound: bool,
}

impl Momentum {
    pub fn new(velocity: Vec3) -> Option<Self> {
        let magnitude = horizontal_square(velocity)?;
        Some(Self {
            last_mag_sq: magnitude,
            stop_mag_tick: 0,
            was_in_air: false,
            end_handler: 0,
            last_tick: None,
            // The source constructor leaves this at Java's zero default.
            last_angle: 0.0,
        })
    }

    /// An existing server handler is not refreshed by addBounceHandler(null).
    /// A predicting client's explicit rebound refreshes only the magnitude.
    pub fn refresh(&mut self, velocity: Vec3, client: bool) {
        if client && let Some(magnitude) = horizontal_square(velocity) {
            self.last_mag_sq = magnitude;
            self.last_angle = atan2(velocity.z, velocity.x);
        }
    }

    pub fn tick(&mut self, tick: i32, motion: Vec3, in_air: bool, cancel: bool) -> Update {
        let Some(motion_sq) = horizontal_square(motion) else {
            return Update {
                active: false,
                velocity: None,
                turn_sound: false,
            };
        };
        if cancel {
            return Update {
                active: false,
                velocity: None,
                turn_sound: false,
            };
        }
        let mut result = Update {
            active: true,
            velocity: None,
            turn_sound: false,
        };
        if self.last_tick == Some(tick) {
            return result;
        }
        self.last_tick = Some(tick);
        if in_air && self.last_mag_sq > 0.0 {
            if motion_sq == 0.0 {
                if self.stop_mag_tick == 0 {
                    self.stop_mag_tick = tick.wrapping_add(5);
                } else if tick > self.stop_mag_tick {
                    self.last_mag_sq = 0.0;
                }
            } else if motion_sq < self.last_mag_sq {
                self.stop_mag_tick = 0;
                let damping = f64::from(0.975f32);
                let boost = (self.last_mag_sq / motion_sq).sqrt() * damping;
                if boost > 1.0 {
                    let velocity = Vec3::new(motion.x * boost, motion.y, motion.z * boost);
                    if horizontal_square(velocity).is_none() {
                        return Update {
                            active: false,
                            velocity: None,
                            turn_sound: false,
                        };
                    }
                    result.velocity = Some(velocity);
                    self.last_mag_sq = self.last_mag_sq * damping * damping;
                    let angle = atan2(motion.z, motion.x);
                    // Source compares the raw angle difference, not a wrapped
                    // shortest arc, and only sounds on an actual correction.
                    result.turn_sound = (angle - self.last_angle).abs() > 1.0;
                    self.last_angle = angle;
                } else {
                    self.last_mag_sq = motion_sq;
                    self.last_angle = atan2(motion.z, motion.x);
                }
            }
        }
        if self.was_in_air && !in_air {
            if self.end_handler == 0 {
                self.end_handler = tick.wrapping_add(5);
            } else if tick > self.end_handler {
                result.active = false;
            }
        } else {
            self.end_handler = 0;
            self.was_in_air = true;
        }
        result
    }
}

fn horizontal_square(velocity: Vec3) -> Option<f64> {
    let square = velocity.x * velocity.x + velocity.z * velocity.z;
    (square.is_finite() && velocity.y.is_finite()).then_some(square)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turn_sound_requires_a_correction_and_uses_raw_source_angle_difference() {
        let mut state = Momentum::new(Vec3::new(1.0, 0.0, 0.0)).unwrap();
        // Too little slowing to boost still updates the angle, without sound.
        assert!(
            !state
                .tick(1, Vec3::new(0.0, 0.0, 0.99), true, false)
                .turn_sound
        );
        assert!(
            !state
                .tick(2, Vec3::new(0.0, 0.0, 0.5), true, false)
                .turn_sound
        );
        assert!(
            state
                .tick(3, Vec3::new(0.5, 0.0, 0.0), true, false)
                .turn_sound
        );
        assert!(
            !state
                .tick(3, Vec3::new(0.0, 0.0, 0.1), true, false)
                .turn_sound
        );
        // Existing client registration resets the angle. Crossing +pi/-pi
        // nevertheless sounds: the source does not normalize that difference.
        state.refresh(Vec3::new(-1.0, 0.0, 0.01), true);
        assert!(
            state
                .tick(4, Vec3::new(-0.5, 0.0, -0.005), true, false)
                .turn_sound
        );
    }

    #[test]
    fn preserves_source_horizontal_fraction_without_vertical_change_or_speed_capture() {
        let mut state = Momentum::new(Vec3::new(1.0, 0.2, 0.0)).unwrap();
        let v = Vec3::new(0.5, -0.2, 0.0);
        let first = state.tick(10, v, true, false);
        assert_eq!(
            first.velocity,
            Some(Vec3::new(f64::from(0.975f32), -0.2, 0.0))
        );
        assert_eq!(state.tick(10, v, true, false).velocity, None);
        assert_eq!(
            state
                .tick(11, Vec3::new(2.0, 0.0, 0.0), true, false)
                .velocity,
            None
        );
        let next = state.tick(12, v, true, false).velocity.unwrap();
        assert!((next.x - f64::from(0.975f32).powi(2)).abs() < 1e-14);
        assert_eq!(next.y, -0.2);
    }

    #[test]
    fn stationary_and_ground_timeouts_keep_the_strict_source_deadline() {
        let mut state = Momentum::new(Vec3::new(1.0, 0.0, 0.0)).unwrap();
        for tick in 20..=26 {
            assert!(state.tick(tick, Vec3::ZERO, true, false).active);
        }
        assert_eq!(
            state
                .tick(27, Vec3::new(0.1, 0.0, 0.0), true, false)
                .velocity,
            None
        );
        for tick in 30..=35 {
            assert!(state.tick(tick, Vec3::ZERO, false, false).active);
        }
        assert!(!state.tick(36, Vec3::ZERO, false, false).active);
    }

    #[test]
    fn water_or_ladder_timeout_resets_on_air_and_gliding_cancels_immediately() {
        let mut state = Momentum::new(Vec3::new(1.0, 0.0, 0.0)).unwrap();
        state.tick(10, Vec3::ZERO, true, false);
        state.tick(11, Vec3::ZERO, false, false);
        state.tick(15, Vec3::ZERO, true, false);
        assert!(state.tick(16, Vec3::ZERO, false, false).active);
        assert!(state.tick(21, Vec3::ZERO, false, false).active);
        assert!(!state.tick(22, Vec3::ZERO, false, false).active);
        assert!(!state.tick(22, Vec3::ZERO, true, true).active);
        assert!(Momentum::new(Vec3::new(f64::NAN, 0.0, 0.0)).is_none());
    }

    #[test]
    fn server_rebounce_keeps_existing_baseline_but_client_refreshes_it() {
        let velocity = Vec3::new(0.5, 0.0, 0.0);
        let mut server = Momentum::new(Vec3::new(1.0, 0.0, 0.0)).unwrap();
        let mut client = server.clone();
        server.refresh(Vec3::new(2.0, 0.0, 0.0), false);
        client.refresh(Vec3::new(2.0, 0.0, 0.0), true);
        let a = server.tick(1, velocity, true, false).velocity.unwrap();
        let b = client.tick(1, velocity, true, false).velocity.unwrap();
        assert_eq!(b.x, a.x * 2.0);
    }
}
