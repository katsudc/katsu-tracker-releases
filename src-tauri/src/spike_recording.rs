//! Full-bandwidth calibration playback with preserved capture channels.
//! The in-memory PCM never goes to disk or a remote service.
#[derive(Default)]
pub struct Recording {
    pub rate: u32,
    pub channels: u16,
    samples: Vec<f32>,
}

impl Recording {
    pub fn append(&mut self, samples: &[f32], rate: u32, channels: u16) -> Result<(), String> {
        if samples.is_empty() { return Ok(()); }
        if channels == 0 || rate == 0 || samples.len() % channels as usize != 0 {
            return Err("Geçersiz ses biçimi.".into());
        }
        if self.samples.is_empty() { self.rate = rate; self.channels = channels; }
        if self.rate != rate || self.channels != channels {
            return Err("Kayıt sırasında ses cihazının biçimi değişti. Yeniden kayıt al.".into());
        }
        // Bound temporary RAM even for unusual multichannel/high-rate devices.
        if self.samples.len().saturating_add(samples.len()) > 16_000_000 {
            return Err("Ses biçimi kayıt sınırını aşıyor. Windows çıkışını 48 kHz stereo seç.".into());
        }
        let limit = self.rate as usize * self.channels as usize * 15;
        let free = limit.saturating_sub(self.samples.len());
        self.samples.extend(samples.iter().take(free).copied());
        Ok(())
    }

    pub fn clear(&mut self) { *self = Self::default(); }

    /// PCM16 WAV at the device's original rate and channel count. Binary IPC
    /// avoids serializing millions of floats into JSON and blocking capture.
    pub fn wav(&self) -> Result<Vec<u8>, String> {
        if self.samples.is_empty() { return Err("Önizleme kaydı bulunamadı. Yeniden kayıt al.".into()); }
        let data_len = (self.samples.len() * 2) as u32;
        let mut out = Vec::with_capacity(44 + data_len as usize);
        out.extend(b"RIFF"); out.extend((36 + data_len).to_le_bytes()); out.extend(b"WAVEfmt ");
        out.extend(16u32.to_le_bytes()); out.extend(1u16.to_le_bytes());
        out.extend(self.channels.to_le_bytes()); out.extend(self.rate.to_le_bytes());
        out.extend((self.rate * self.channels as u32 * 2).to_le_bytes());
        out.extend((self.channels * 2).to_le_bytes()); out.extend(16u16.to_le_bytes());
        out.extend(b"data"); out.extend(data_len.to_le_bytes());
        for &v in &self.samples {
            let v = if v.is_finite() { v } else { 0.0 };
            let pcm = (v * 32768.0).round().clamp(-32768.0, 32767.0) as i16;
            out.extend(pcm.to_le_bytes());
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_high_frequencies_stereo_timing_and_native_rate() {
        for rate in [44_100, 48_000, 96_000] {
            let samples: Vec<f32> = (0..rate/10).flat_map(|i| {
                let t = i as f32 / rate as f32;
                [0.4*(std::f32::consts::TAU*10_000.0*t).sin(), 0.2*(std::f32::consts::TAU*6_000.0*t).cos()]
            }).collect();
            let mut recording = Recording::default();
            for packet in samples.chunks(274) { recording.append(packet,rate,2).unwrap(); }
            let wav = recording.wav().unwrap();
            assert_eq!(&wav[0..4], b"RIFF"); assert_eq!(&wav[8..12], b"WAVE");
            assert_eq!(u32::from_le_bytes(wav[24..28].try_into().unwrap()), rate);
            assert_eq!(u16::from_le_bytes(wav[22..24].try_into().unwrap()), 2);
            assert_eq!(wav.len(), 44 + samples.len()*2);
            for (bytes, original) in wav[44..].chunks_exact(2).zip(samples) {
                let decoded = i16::from_le_bytes(bytes.try_into().unwrap()) as f32 / 32768.0;
                assert!((decoded-original).abs() <= 1.0/32768.0);
            }
        }
    }
    #[test]
    fn rejects_format_changes_and_drops_audio_on_clear() {
        let mut recording = Recording::default();
        recording.append(&[0.1, -0.1],48_000,2).unwrap();
        assert!(recording.append(&[0.1,-0.1],44_100,2).is_err());
        assert!(recording.append(&[0.1],48_000,2).is_err());
        recording.clear();
        assert!(recording.wav().is_err());
        recording.append(&[0.2],44_100,1).unwrap();
    }
    #[test]
    fn safely_encodes_clipped_nonfinite_and_silent_samples() {
        let mut recording = Recording::default();
        recording.append(&[-2.0,2.0,f32::NAN,f32::INFINITY,0.0],48_000,1).unwrap();
        let wav = recording.wav().unwrap();
        let values: Vec<_> = wav[44..].chunks_exact(2).map(|b|i16::from_le_bytes(b.try_into().unwrap())).collect();
        assert_eq!(values, vec![-32768,32767,0,0,0]);
    }
}
