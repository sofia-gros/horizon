// Adapted from Tinkers' Construct ModifierEvents.bounceOnFall,
// Copyright (c) 2022 SlimeKnights, MIT License.
// Full notice in THIRD_PARTY_NOTICES.md. Changed: pure opt-in player kernel.
//! Explicit mod landing response, separate from vanilla block restitution.
use horizon_core::geometry::Vec3;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Response {
    Unchanged,
    Suppressed,
    Bounce(Vec3),
}

pub fn response(
    enabled: bool,
    distance: f64,
    velocity: Vec3,
    step_height: f64,
    gravity: f64,
    suppressing: bool,
    client: bool,
) -> Response {
    if !enabled
        || !distance.is_finite()
        || !velocity.y.is_finite()
        || !step_height.is_finite()
        || (distance < 3.0 && velocity.y > -0.3)
        || distance <= 0.5 + (step_height - 0.6)
    {
        return Response::Unchanged;
    }
    if suppressing {
        return Response::Suppressed;
    }
    if !gravity.is_finite() || gravity <= 0.0 {
        return Response::Unchanged;
    }
    let y = if client {
        velocity.y * f64::from(if distance < 2.0 { -0.7_f32 } else { -0.9_f32 })
    } else {
        // Keep source operation order: gravity * sqrt(distance / gravity).
        gravity * (distance / gravity).sqrt()
    };
    Response::Bounce(Vec3::new(
        velocity.x / f64::from(0.975_f32),
        y,
        velocity.z / f64::from(0.975_f32),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opt_in_thresholds_crouching_and_server_velocity_are_distinct() {
        let v = Vec3::new(0.39, -1.0, -0.195);
        assert_eq!(
            response(false, 8.0, v, 0.6, 0.08, false, false),
            Response::Unchanged
        );
        assert_eq!(
            response(true, 0.5, v, 0.6, 0.08, false, false),
            Response::Unchanged
        );
        assert_eq!(
            response(true, 1.0, v, 1.2, 0.08, false, false),
            Response::Unchanged
        );
        assert_eq!(
            response(true, 2.0, Vec3::ZERO, 0.6, 0.08, false, false),
            Response::Unchanged
        );
        assert_eq!(
            response(true, 8.0, v, 0.6, 0.08, true, false),
            Response::Suppressed
        );
        let Response::Bounce(out) = response(true, 8.0, v, 0.6, 0.08, false, false) else {
            panic!("no bounce")
        };
        assert_eq!(out.y, 0.8);
        assert!((out.x - 0.4).abs() < 1e-7);
        assert!((out.z + 0.2).abs() < 1e-7);
        let Response::Bounce(client) = response(true, 8.0, v, 0.6, 0.08, false, true) else {
            panic!("no client bounce")
        };
        assert_eq!(client.y, f64::from(0.9_f32));
        assert_eq!(
            response(true, 8.0, v, 0.6, 0.0, false, false),
            Response::Unchanged
        );
    }
}
