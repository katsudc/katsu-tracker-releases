//! Windows WASAPI render-loopback. Samples stay in this process.
use super::spike_signal::RATE;
use windows::Win32::Media::Audio::*;
use windows::Win32::System::Com::{CoCreateInstance, CoTaskMemFree, CLSCTX_ALL};

pub struct Packet {
    pub samples: Vec<f32>, pub discontinuity: bool,
    pub native_samples: Vec<f32>, pub rate: u32, pub channels: u16,
}

pub struct Capture {
    client: IAudioClient,
    reader: IAudioCaptureClient,
    channels: usize,
    rate: u32,
    bits: u16,
    float: bool,
}

impl Capture {
    pub fn open() -> Result<Self, String> {
        unsafe {
            let devices: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(|e| e.to_string())?;
            let device = devices.GetDefaultAudioEndpoint(eRender, eConsole).map_err(|e| e.to_string())?;
            let client: IAudioClient = device.Activate(CLSCTX_ALL, None).map_err(|e| e.to_string())?;
            let format = client.GetMixFormat().map_err(|e| e.to_string())?;
            let mut base = format.read_unaligned();
            let bits = base.wBitsPerSample;
            let tag = if base.wFormatTag == 0xfffe && base.cbSize >= 22 {
                (*(format as *const WAVEFORMATEXTENSIBLE)).SubFormat.data1 as u16
            } else { base.wFormatTag };
            let valid = base.nChannels > 0 && base.nChannels <= 32 && base.nSamplesPerSec >= 8_000
                && ((tag == 3 && bits == 32) || (tag == 1 && [16,24,32].contains(&bits)))
                && base.nBlockAlign as usize == base.nChannels as usize * (base.wBitsPerSample / 8) as usize;
            if !valid {
                CoTaskMemFree(Some(format.cast()));
                return Err("Ses cihazının biçimi desteklenmiyor. Windows'ta 44,1 veya 48 kHz PCM seç.".into());
            }
            // Request 48 kHz from Windows; preserve the endpoint channel layout.
            // Windows performs any device-rate conversion before capture.
            base.nSamplesPerSec = RATE as u32;
            base.nAvgBytesPerSec = RATE as u32 * base.nBlockAlign as u32;
            format.write_unaligned(base);
            let flags = AUDCLNT_STREAMFLAGS_LOOPBACK | AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM
                | AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY;
            let initialized = client.Initialize(AUDCLNT_SHAREMODE_SHARED, flags, 1_000_000, 0, format, None);
            CoTaskMemFree(Some(format.cast()));
            initialized.map_err(|e| e.to_string())?;
            let reader = client.GetService().map_err(|e| e.to_string())?;
            client.Start().map_err(|e| e.to_string())?;
            Ok(Self { client, reader, channels:base.nChannels as usize, rate:base.nSamplesPerSec,
                bits:base.wBitsPerSample, float:tag==3 })
        }
    }

    pub fn read(&mut self, record_preview: bool) -> Result<Packet, String> {
        let mut packet = Packet { samples:Vec::new(), discontinuity:false,
            native_samples:Vec::new(), rate:self.rate, channels:self.channels as u16 };
        unsafe {
            while self.reader.GetNextPacketSize().map_err(|e| e.to_string())? > 0 {
                let (mut data, mut frames, mut flags) = (std::ptr::null_mut(), 0, 0);
                self.reader.GetBuffer(&mut data, &mut frames, &mut flags, None, None).map_err(|e| e.to_string())?;
                packet.discontinuity |= flags & AUDCLNT_BUFFERFLAGS_DATA_DISCONTINUITY.0 as u32 != 0;
                let silent = flags & AUDCLNT_BUFFERFLAGS_SILENT.0 as u32 != 0;
                let bytes_per_sample = (self.bits / 8) as usize;
                for frame in 0..frames as usize {
                    let mut mono = 0.0;
                    if !silent && !data.is_null() {
                        for channel in 0..self.channels {
                            let p = data.add((frame * self.channels + channel) * bytes_per_sample);
                            let value = if self.float { (p as *const f32).read_unaligned() } else {
                                match self.bits {
                                    16 => (p as *const i16).read_unaligned() as f32 / 32768.0,
                                    24 => (((*p.add(2) as i32) << 24 | (*p.add(1) as i32) << 16 | (*p as i32) << 8) >> 8) as f32 / 8388608.0,
                                    _ => (p as *const i32).read_unaligned() as f32 / 2147483648.0,
                                }
                            };
                            let value = if value.is_finite() { value } else { 0.0 };
                            mono += value;
                            if record_preview { packet.native_samples.push(value); }
                        }
                        mono /= self.channels as f32;
                    } else if record_preview {
                        packet.native_samples.extend(std::iter::repeat(0.0).take(self.channels));
                    }
                    packet.samples.push(mono);
                }
                self.reader.ReleaseBuffer(frames).map_err(|e| e.to_string())?;
                // Bound work if the device accumulated data during a stalled frame.
                if packet.samples.len() > RATE { packet.discontinuity = true; break; }
            }
        }
        Ok(packet)
    }
}

impl Drop for Capture { fn drop(&mut self) { unsafe { let _ = self.client.Stop(); } } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires an active Windows audio endpoint"]
    fn native_capture_opens_at_48khz() {
        use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_MULTITHREADED};
        unsafe { CoInitializeEx(None, COINIT_MULTITHREADED).ok().unwrap(); }
        {
            let mut capture = Capture::open().unwrap();
            assert_eq!(capture.rate,48000);
            let packet = capture.read(false).unwrap();
            assert_eq!(packet.rate,48000);
            assert!(packet.native_samples.is_empty());
        }
        unsafe { CoUninitialize(); }
    }
}
