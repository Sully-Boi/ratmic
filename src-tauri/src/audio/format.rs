//! Sample format conversion + simple downmix helpers.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioFormat {
    pub sample_rate: u32,
    pub channels: u16,
}

/// Downmix interleaved multi-channel f32 samples to mono in-place.
/// Returns the number of mono samples written.
pub fn downmix_to_mono(input: &[f32], channels: u16, out: &mut Vec<f32>) -> usize {
    out.clear();
    if channels <= 1 {
        out.extend_from_slice(input);
        return input.len();
    }
    let ch = channels as usize;
    let frames = input.len() / ch;
    out.reserve(frames);
    for f in 0..frames {
        let mut sum = 0.0;
        for c in 0..ch {
            sum += input[f * ch + c];
        }
        out.push(sum / ch as f32);
    }
    frames
}

/// Upmix mono samples to interleaved n-channel by duplicating each sample.
pub fn mono_to_interleaved(input: &[f32], channels: u16, out: &mut Vec<f32>) {
    out.clear();
    let ch = channels as usize;
    out.reserve(input.len() * ch);
    for &s in input {
        for _ in 0..ch {
            out.push(s);
        }
    }
}

/// Convert i16 PCM to f32 in [-1.0, 1.0].
pub fn i16_to_f32(input: &[i16], out: &mut Vec<f32>) {
    out.clear();
    out.reserve(input.len());
    let scale = 1.0 / 32768.0_f32;
    for &s in input {
        out.push(s as f32 * scale);
    }
}

/// Convert f32 in [-1.0, 1.0] to i16 PCM with hard clipping.
pub fn f32_to_i16(input: &[f32], out: &mut Vec<i16>) {
    out.clear();
    out.reserve(input.len());
    for &s in input {
        let clipped = s.clamp(-1.0, 1.0);
        out.push((clipped * 32767.0) as i16);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mono_passthrough() {
        let mut out = Vec::new();
        let n = downmix_to_mono(&[0.1, 0.2, 0.3], 1, &mut out);
        assert_eq!(n, 3);
        assert_eq!(out, vec![0.1, 0.2, 0.3]);
    }

    #[test]
    fn stereo_downmix_averages_channels() {
        let mut out = Vec::new();
        let n = downmix_to_mono(&[1.0, -1.0, 0.5, -0.5], 2, &mut out);
        assert_eq!(n, 2);
        assert!((out[0]).abs() < 1e-6);
        assert!((out[1]).abs() < 1e-6);
    }

    #[test]
    fn upmix_duplicates() {
        let mut out = Vec::new();
        mono_to_interleaved(&[0.1, 0.2], 2, &mut out);
        assert_eq!(out, vec![0.1, 0.1, 0.2, 0.2]);
    }

    #[test]
    fn i16_round_trip_near_unity() {
        let original_f = vec![0.0, 0.5, -0.5, 1.0, -1.0];
        let mut as_i16: Vec<i16> = Vec::new();
        f32_to_i16(&original_f, &mut as_i16);
        let mut back_f = Vec::new();
        i16_to_f32(&as_i16, &mut back_f);
        for (a, b) in original_f.iter().zip(back_f.iter()) {
            assert!((a - b).abs() < 1e-3, "round-trip drift: {} vs {}", a, b);
        }
    }

    #[test]
    fn f32_to_i16_clamps_extremes() {
        let mut out = Vec::new();
        f32_to_i16(&[2.0, -2.0], &mut out);
        assert_eq!(out, vec![32767, -32767]);
    }
}
