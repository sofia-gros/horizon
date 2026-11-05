// Adapted from Tinkers' Construct SlimeBounceHandler,
// Copyright (c) 2022 SlimeKnights, MIT License.
// Full notice in THIRD_PARTY_NOTICES.md. Changed: pure player momentum state,
// finite-input and once-per-host-tick guards; landing restitution is separate.
//! Airborne continuation after an explicitly enabled effect rebound.
use horizon_core::geometry::Vec3;

#[derive(Clone, Debug)]
pub struct Momentum {
    last_mag_sq: f64,
    stop_mag_tick: i32,
    was_in_air: bool,
    end_handler: i32,
    last_tick: Option<i32>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Update {
    pub active: bool,
    pub velocity: Option<Vec3>,
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
        })
    }

    /// An existing server handler is not refreshed by addBounceHandler(null).
    /// A predicting client's explicit rebound refreshes only the magnitude.
    pub fn refresh(&mut self, velocity: Vec3, client: bool) {
        if client && let Some(magnitude) = horizontal_square(velocity) {
            self.last_mag_sq = magnitude;
        }
    }

    pub fn tick(&mut self, tick: i32, motion: Vec3, in_air: bool, cancel: bool) -> Update {
        let Some(motion_sq) = horizontal_square(motion) else {
            return Update {
                active: false,
                velocity: None,
            };
        };
        if cancel {
            return Update {
                active: false,
                velocity: None,
            };
        }
        let mut result = Update {
            active: true,
            velocity: None,
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
                        };
                    }
                    result.velocity = Some(velocity);
                    self.last_mag_sq = self.last_mag_sq * damping * damping;
                } else {
                    self.last_mag_sq = motion_sq;
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
