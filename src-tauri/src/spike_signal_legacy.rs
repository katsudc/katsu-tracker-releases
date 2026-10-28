//! Local spectral template matching. No game memory, microphone or remote service.
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

pub const RATE: usize = 8_000;
const WINDOW: usize = 512;
const HOP: usize = 160;
const BANDS: usize = 24;
const FRAMES: usize = 50;
pub const TEMPLATE_SAMPLES: usize = WINDOW + HOP * (FRAMES - 1);
pub const TEMPLATE_SECONDS: f64 = TEMPLATE_SAMPLES as f64 / RATE as f64;

#[derive(Clone, Serialize, Deserialize)]
pub struct Template {
    pub frames: Vec<Vec<f32>>,
}

fn spectrum(samples: &[f32]) -> Vec<f32> {
    // Analysis-only gain keeps quiet spectra above the numerical power floor.
    // It never changes playback volume and cannot recover a fully masked sound.
    let rms = (samples.iter().map(|x| x*x).sum::<f32>() / samples.len() as f32).sqrt();
    let gain = (0.01 / rms.max(1e-7)).clamp(1.0, 10_000.0);
    spectrum_at_gain(samples, gain)
}

fn spectrum_at_gain(samples: &[f32], gain: f32) -> Vec<f32> {
    static WEIGHTS: std::sync::OnceLock<([f32; BANDS], [f32; WINDOW])> = std::sync::OnceLock::new();
    let (coefficients, hann) = WEIGHTS.get_or_init(|| {
        (std::array::from_fn(|band| {
            let hz = 180.0_f32 * 20.0_f32.powf(band as f32 / (BANDS - 1) as f32);
            2.0 * (std::f32::consts::TAU * hz / RATE as f32).cos()
        }), std::array::from_fn(|i| 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / (WINDOW - 1) as f32).cos()))
    });
    let mut result = Vec::with_capacity(BANDS);
    for &coefficient in coefficients {
        let (mut prev, mut prev2) = (0.0_f32, 0.0_f32);
        for (i, value) in samples.iter().enumerate() {
            let current = value * gain * hann[i] + coefficient * prev - prev2;
            prev2 = prev;
            prev = current;
        }
        let power = (prev * prev + prev2 * prev2 - coefficient * prev * prev2).max(1e-12);
        result.push(power.ln());
    }
    let mean = result.iter().sum::<f32>() / BANDS as f32;
    for v in &mut result { *v -= mean; }
    let norm = result.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm > 1e-5 { for v in &mut result { *v /= norm; } }
    result
}

impl Template {
    pub fn from_audio(samples: &[f32], marker_seconds: f64) -> Result<Self, String> {
        if !marker_seconds.is_finite() || marker_seconds < 0.0 {
            return Err("Geçerli bir kurulma anı seç.".into());
        }
        let start = (marker_seconds * RATE as f64).round() as usize;
        let audio = samples.get(start..start.saturating_add(TEMPLATE_SAMPLES))
            .ok_or("İşaretten sonra en az 1,1 saniyelik ses gerekli.")?;
        let rms = (audio.iter().map(|x| x * x).sum::<f32>() / audio.len() as f32).sqrt();
        if !rms.is_finite() || rms < 0.0003 {
            return Err("Seçilen bölüm çok sessiz. Kurulma sesinin olduğu yeri seç.".into());
        }
        let frames = (0..FRAMES).map(|i| spectrum(&audio[i * HOP..i * HOP + WINDOW])).collect();
        Ok(Self { frames })
    }

    pub fn valid(&self) -> bool {
        self.frames.len() == FRAMES && self.frames.iter().all(|f|
            f.len() == BANDS && f.iter().all(|x| x.is_finite() && x.abs() <= 1.01))
    }
}

pub struct Detector {
    audio: VecDeque<f32>,
    frames: VecDeque<Vec<f32>>,
    until_hop: usize,
}

impl Default for Detector {
    fn default() -> Self {
        Self { audio: VecDeque::new(), frames: VecDeque::new(), until_hop: WINDOW }
    }
}

impl Detector {
    // Allow a 20 ms local timing difference instead of requiring every noisy
    // audio frame to line up exactly. The saved spectral templates stay valid.
    fn similarity(&self, template: &Template) -> f32 {
        let scores: Vec<f32> = template.frames.iter().enumerate().map(|(i, reference)| {
            let start = i.saturating_sub(1);
            let end = (i + 1).min(FRAMES - 1);
            (start..=end).map(|j| self.frames[j].iter().zip(reference)
                .map(|(x,y)| x*y).sum::<f32>()).fold(-1.0_f32, f32::max)
        }).collect();
        let mean = scores.iter().sum::<f32>() / FRAMES as f32;
        // A brief overlapping effect may obscure part of the sequence. Require
        // evidence across two thirds of the timeline and 60% of all frames
        // before discounting the worst 24%. Bound the correction to 0.12.
        let matching = scores.iter().filter(|&&v| v > 0.65).count();
        let regions = scores.chunks(17).filter(|part|
            part.iter().sum::<f32>() / part.len() as f32 > 0.65).count();
        if matching < 30 || regions < 2 { return mean; }
        let mut sorted = scores;
        sorted.sort_by(f32::total_cmp);
        let robust = sorted[12..].iter().sum::<f32>() / (FRAMES - 12) as f32;
        robust.min(mean + 0.12)
    }
    /// Returns (best similarity, sample position of that window's end).
    pub fn push(&mut self, samples: &[f32], template: &Template) -> Option<(f32, usize)> {
        let mut best = None;
        for (position, sample) in samples.iter().enumerate() {
            self.audio.push_back(if sample.is_finite() { *sample } else { 0.0 });
            if self.audio.len() > WINDOW { self.audio.pop_front(); }
            self.until_hop -= 1;
            if self.until_hop != 0 { continue; }
            self.until_hop = HOP;
            let frame: Vec<f32> = self.audio.iter().copied().collect();
            let rms = (frame.iter().map(|x| x*x).sum::<f32>() / WINDOW as f32).sqrt();
            self.frames.push_back(if rms < 1e-7 { vec![0.0; BANDS] } else { spectrum(&frame) });
            if self.frames.len() > FRAMES { self.frames.pop_front(); }
            if self.frames.len() != FRAMES { continue; }
            let score = self.similarity(template);
            if best.map_or(true, |(value, _)| score > value) { best = Some((score, position + 1)); }
        }
        best
    }
}

pub const FUSE_SECONDS: f64 = 45.0;
pub const FULL_DEFUSE_SECONDS: f64 = 7.0;
pub const HALF_DEFUSE_SECONDS: f64 = 3.5;

pub fn remaining(elapsed: f64) -> f64 { (FUSE_SECONDS - elapsed.max(0.0)).clamp(0.0, FUSE_SECONDS) }

#[cfg(test)]
mod tests {
    use super::*;
    fn sound(length: usize, shift: f32) -> Vec<f32> {
        (0..length).map(|i| {
            let t = i as f32 / RATE as f32;
            let hz = 350.0 + (t * 5.0).floor() * 410.0 + shift;
            0.1 * (std::f32::consts::TAU * hz * t).sin()
        }).collect()
    }
    fn fixture() -> Vec<f32> {
        include_bytes!("../assets/spike-planted-reference.pcm16").chunks_exact(2)
            .map(|b| i16::from_le_bytes([b[0],b[1]]) as f32 / 32768.0).collect()
    }
    fn peak(audio: &[f32], template: &Template) -> f32 {
        let mut detector = Detector::default();
        let mut best = 0.0_f32;
        for chunk in audio.chunks(137) {
            if let Some((score,_)) = detector.push(chunk, template) { best = best.max(score); }
        }
        best
    }
    fn noise(length: usize) -> Vec<f32> {
        let mut seed = 156_u32;
        (0..length).map(|_| {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            (seed as f64 / u32::MAX as f64 * 2.0 - 1.0) as f32
        }).collect()
    }
    #[test]
    fn detects_very_quiet_reference_without_changing_saved_template() {
        let audio = fixture();
        let template = Template { frames: (0..FRAMES).map(|i|
            spectrum_at_gain(&audio[i*HOP..i*HOP+WINDOW],1.0)).collect() };
        let saved = serde_json::to_string(&template).unwrap();
        let restored: Template = serde_json::from_str(&saved).unwrap();
        for gain in [0.01,0.001,0.0001] {
            let quiet: Vec<_> = audio.iter().map(|v|v*gain).collect();
            let score = peak(&quiet,&restored);
            assert!(score > 0.95,"gain {gain}: {score}");
        }
    }
    #[test]
    fn detects_reference_with_short_loud_overlapping_burst() {
        let audio = fixture();
        let template = Template::from_audio(&audio,0.0).unwrap();
        let interference = noise(audio.len());
        for (start,duration) in [(0.1,0.15),(0.38,0.30),(0.70,0.20)] {
            let mut mixed = audio.clone();
            for i in (start * RATE as f32) as usize..((start+duration)*RATE as f32) as usize {
                mixed[i] += interference[i]*0.8;
            }
            let score = peak(&mixed,&template);
            assert!(score >= 0.82,"burst {start}/{duration}: {score}");
        }
    }
    #[test]
    fn rejects_noise_bursts_reversed_sound_and_activation_at_user_threshold() {
        let audio = fixture();
        let template = Template::from_audio(&audio,0.0).unwrap();
        let mut reversed = audio.clone(); reversed.reverse();
        let noise = noise(RATE*4);
        let bursts: Vec<_> = noise.iter().enumerate().map(|(i,v)|
            if i % 2400 < 400 { *v } else { *v*0.00001 }).collect();
        let activation: Vec<_> = include_bytes!("../tests/fixtures/spike-activation-negative.pcm16")
            .chunks_exact(2).map(|b|i16::from_le_bytes([b[0],b[1]]) as f32/32768.0).collect();
        for (name,signal) in [("noise",noise),("bursts",bursts),("reversed",reversed),("activation",activation)] {
            let score = peak(&signal,&template);
            assert!(score < 0.75,"negative {name}: {score}");
        }
    }
    #[test]
    fn matches_reference_after_silence_and_with_lower_volume() {
        let audio = sound(TEMPLATE_SAMPLES, 0.0);
        let template = Template::from_audio(&audio, 0.0).unwrap();
        let mut detector = Detector::default();
        detector.push(&vec![0.0; HOP * 5], &template);
        let quieter: Vec<f32> = audio.iter().map(|x| x * 0.3).collect();
        let (score, end) = detector.push(&quieter, &template).unwrap();
        assert!(score > 0.97, "score: {score}");
        // Adjacent windows can tie after rejecting obscured frames; the
        // allowed local alignment tolerance is one 20 ms hop.
        assert!(end.abs_diff(TEMPLATE_SAMPLES) <= HOP, "end: {end}");
    }
    #[test]
    fn rejects_silence_and_unrelated_tones() {
        let template = Template::from_audio(&sound(TEMPLATE_SAMPLES, 0.0), 0.0).unwrap();
        let silent = Detector::default().push(&vec![0.0; TEMPLATE_SAMPLES], &template).unwrap().0;
        let other = Detector::default().push(&sound(TEMPLATE_SAMPLES, 1700.0), &template).unwrap().0;
        assert!(silent < 0.1);
        assert!(other < 0.80, "unrelated score: {other}");
        assert!(Template::from_audio(&vec![0.0; TEMPLATE_SAMPLES], 0.0).is_err());
    }
    #[test]
    fn matches_across_packet_boundaries_and_frame_offsets() {
        let reference = sound(TEMPLATE_SAMPLES, 0.0);
        let template = Template::from_audio(&reference, 0.0).unwrap();
        for offset in [0, 79, 159, 257] {
            let mut audio = vec![0.0; offset];
            audio.extend(&reference);
            audio.extend(vec![0.0; HOP * 2]);
            let mut detector = Detector::default();
            let mut best = 0.0_f32;
            for packet in audio.chunks(137) {
                if let Some((score, _)) = detector.push(packet, &template) { best = best.max(score); }
            }
            assert!(best >= 0.9, "offset {offset}: {best}");
        }
    }
    #[test]
    fn rejects_invalid_calibration_and_does_not_extend_timer() {
        assert!(Template::from_audio(&[0.1; 10], 0.0).is_err());
        assert!(Template::from_audio(&[0.1; 10], f64::NAN).is_err());
        assert_eq!(remaining(0.0), 45.0);
        assert_eq!(remaining(38.0), FULL_DEFUSE_SECONDS);
        assert_eq!(remaining(41.5), HALF_DEFUSE_SECONDS);
        assert_eq!(remaining(49.0), 0.0);
    }
    #[test]
    fn tolerates_local_frame_jitter_but_rejects_different_pattern() {
        let frames: Vec<Vec<f32>> = (0..FRAMES).map(|i| {
            let mut v = vec![0.0; BANDS]; v[i % BANDS] = 1.0; v
        }).collect();
        let template = Template {frames:frames.clone()};
        let shifted: VecDeque<_> = (0..FRAMES).map(|i|frames[i ^ 1].clone()).collect();
        let strict = shifted.iter().zip(&template.frames)
            .map(|(a,b)| a.iter().zip(b).map(|(x,y)| x*y).sum::<f32>()).sum::<f32>() / FRAMES as f32;
        let mut detector = Detector::default(); detector.frames = shifted;
        assert!(strict < 0.9);
        assert!(detector.similarity(&template) > 0.99);
        detector.frames = (0..FRAMES).map(|i|frames[(i+7)%FRAMES].clone()).collect();
        assert!(detector.similarity(&template) < 0.7);
    }
}
