// Adapted from Mantle TextureColorHelper.computeAverageColor at
// 17c287ad52f7b10154e07fa7570dc7bccd74ca2b, Copyright (c) 2022 SlimeKnights.
// That method is explicitly licensed under the Apache License, Version 2.0.
// You may obtain a copy at http://www.apache.org/licenses/LICENSE-2.0
// See licenses/mantle-texture-color-Apache-2.0.txt and THIRD_PARTY_NOTICES.md.
// Distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND.
// Changes: Rust RGBA slices, bounds checks, RGB-only result, and pack-load cache.

/// Source weighted frame-zero RGB. Preserve x-major f32 accumulation and the
/// source's unusual RGBtoHSB(alpha, red, green) arguments: blue affects the
/// brightness threshold and weighted output, but not saturation weight.
pub(crate) fn average(side: u32, rgba: &[u8]) -> Option<u32> {
    let side = usize::try_from(side).ok()?;
    if side == 0 || side.checked_mul(side)?.checked_mul(4)? != rgba.len() {
        return None;
    }
    let (mut red, mut green, mut blue, mut count) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
    for x in 0..side {
        for y in 0..side {
            let offset = (y * side + x) * 4;
            let (r, g, b, a) = (
                rgba[offset],
                rgba[offset + 1],
                rgba[offset + 2],
                rgba[offset + 3],
            );
            if a > 0x7f && r.max(g).max(b) > 0x1f {
                let max = a.max(r).max(g);
                let min = a.min(r).min(g);
                let weight = f32::from(max - min) / f32::from(max);
                red += f32::from(r) * weight;
                green += f32::from(g) * weight;
                blue += f32::from(b) * weight;
                count += weight;
            }
        }
    }
    if count > 0.0 {
        red /= count;
        green /= count;
        blue /= count;
    }
    Some((red as u32) << 16 | (green as u32) << 8 | blue as u32)
}

#[cfg(test)]
mod tests {
    use super::average;

    #[test]
    fn source_thresholds_weights_and_empty_result_are_preserved() {
        assert_eq!(average(1, &[255, 0, 0, 255]), Some(0xff0000));
        assert_eq!(average(1, &[255, 0, 0, 127]), Some(0));
        assert_eq!(average(1, &[31, 31, 31, 255]), Some(0));
        assert_eq!(average(1, &[32, 0, 0, 128]), Some(0x200000));
        // Source passes alpha/red/green to RGBtoHSB. Opaque yellow and white
        // therefore have zero weight, rather than yellow receiving saturation 1.
        assert_eq!(
            average(
                2,
                &[
                    255, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255, 0, 0, 0, 255
                ]
            ),
            Some(0x0000ff)
        );
        assert_eq!(
            average(2, &[255, 0, 0, 255, 0, 0, 255, 255, 0, 0, 0, 0, 0, 0, 0, 0]),
            Some(0x7f007f)
        );
        assert_eq!(average(1, &[255, 255, 255, 255]), Some(0));
        assert_eq!(average(0, &[]), None);
        assert_eq!(average(2, &[255; 4]), None);
    }
}
