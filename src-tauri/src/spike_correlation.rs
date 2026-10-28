//! FFT matched filter with LOCAL (per lag) zero-mean energy normalization.
//! Capture supplies continuous 48 kHz mono, including silence; no input synthesis.
use base64::{engine::general_purpose::STANDARD, Engine};
use rustfft::{num_complex::Complex, Fft, FftPlanner};
use serde::{Deserialize, Serialize};
use std::{collections::VecDeque, sync::Arc};
use crate::spike_signal::RATE;
use crate::spike_sequence::Hit;

const LENGTH: usize = RATE / 2;
const STEP: usize = RATE / 10;
const HISTORY: usize = LENGTH + STEP - 1;
const FFT_SIZE: usize = 65536;

#[derive(Clone, Serialize, Deserialize)]
pub struct Waveform {
    pub pcm16: String,
    pub completion_seconds: f64,
}

impl Waveform {
    pub fn from_audio(samples: &[f32], completion_seconds: f64) -> Self {
        let bytes:Vec<_>=samples.iter().flat_map(|x| {
            let value=if x.is_finite() {*x} else {0.0};
            ((value*32768.0).round().clamp(-32768.0,32767.0) as i16).to_le_bytes()
        }).collect();
        Self {pcm16:STANDARD.encode(bytes),completion_seconds}
    }
    pub fn decode(&self)->Option<Vec<f32>> {
        if !self.completion_seconds.is_finite() || !(0.0..=8.0).contains(&self.completion_seconds)
            || self.pcm16.len()>1_200_000 {return None;}
        let bytes=STANDARD.decode(&self.pcm16).ok()?;
        if bytes.len()%2!=0 || bytes.len()>RATE*10*2
            || bytes.len()/2 < (self.completion_seconds*RATE as f64).round() as usize+RATE {return None;}
        Some(bytes.chunks_exact(2).map(|b|i16::from_le_bytes([b[0],b[1]]) as f32/32768.0).collect())
    }
}

struct Kernel { spectrum:Vec<Complex<f32>>, reference:Vec<f32>, elapsed:f64 }
struct Filter {
    forward:Arc<dyn Fft<f32>>, inverse:Arc<dyn Fft<f32>>, kernels:Vec<Kernel>,
    input:Vec<Complex<f32>>, product:Vec<Complex<f32>>, scratch:Vec<Complex<f32>>,
}
impl Filter {
    fn new(wave:&Waveform)->Option<Self> {
        let audio=wave.decode()?;
        let completion=(wave.completion_seconds*RATE as f64).round() as usize;
        let mut planner=FftPlanner::<f32>::new();
        let forward=planner.plan_fft_forward(FFT_SIZE);
        let inverse=planner.plan_fft_inverse(FFT_SIZE);
        let mut scratch=vec![Complex::default();forward.get_inplace_scratch_len().max(inverse.get_inplace_scratch_len())];
        let mut kernels=vec![];
        // Only post-completion waveform fragments can trigger. Preparation is
        // handled separately by the spectral sequence's confirmation gate.
        for offset in [0,RATE/4,RATE/2] {
            let reference=audio.get(completion+offset..completion+offset+LENGTH)?;
            let mean=reference.iter().map(|&x|x as f64).sum::<f64>()/LENGTH as f64;
            let energy=reference.iter().map(|&x|(x as f64-mean).powi(2)).sum::<f64>();
            if (energy/LENGTH as f64)<1e-12 {continue;}
            let mut spectrum=vec![Complex::default();FFT_SIZE];
            for (i,&x) in reference.iter().rev().enumerate() {
                spectrum[i].re=((x as f64-mean)/energy.sqrt()) as f32;
            }
            forward.process_with_scratch(&mut spectrum,&mut scratch);
            kernels.push(Kernel {spectrum,reference:reference.to_vec(),elapsed:(offset+LENGTH) as f64/RATE as f64});
        }
        if kernels.is_empty() {return None;}
        Some(Self {forward,inverse,kernels,input:vec![Complex::default();FFT_SIZE],
            product:vec![Complex::default();FFT_SIZE],scratch})
    }
    fn scan(&mut self, audio:&VecDeque<f32>)->Vec<(f32,f64)> {
        self.input.fill(Complex::default());
        let mut sum=vec![0.0_f64;audio.len()+1];let mut square=sum.clone();
        for (i,&x) in audio.iter().enumerate() {
            self.input[i].re=x;sum[i+1]=sum[i]+x as f64;square[i+1]=square[i]+(x as f64).powi(2);
        }
        self.forward.process_with_scratch(&mut self.input,&mut self.scratch);
        let mut results=vec![];
        for kernel in &self.kernels {
            let mut candidate=(0.0_f32,0_usize);
            for ((out,input),reference) in self.product.iter_mut().zip(&self.input).zip(&kernel.spectrum) {
                *out=*input * *reference;
            }
            self.inverse.process_with_scratch(&mut self.product,&mut self.scratch);
            for start in 0..=audio.len()-LENGTH {
                let end=start+LENGTH;
                let total=sum[end]-sum[start];
                let energy=(square[end]-square[start]-total*total/LENGTH as f64).max(0.0);
                if (energy/LENGTH as f64)<1e-14 {continue;}
                let score=(self.product[end-1].re as f64/FFT_SIZE as f64/energy.sqrt()).abs().min(1.0) as f32;
                if score>candidate.0 {candidate=(score,start);}
            }
            results.push(if distributed_evidence(audio,candidate.1,&kernel.reference) {
                (candidate.0,kernel.elapsed+(audio.len()-candidate.1-LENGTH) as f64/RATE as f64)
            } else {(0.0,0.0)});
        }
        results
    }
}

// A single transient or half a recording followed by silence can produce a
// large global NCC. Require evidence over at least three quarters AND the tail.
fn distributed_evidence(audio:&VecDeque<f32>,start:usize,reference:&[f32])->bool {
    let mut matched=0;let mut tail=false;
    for part in 0..4 {
        let n=LENGTH/4;let offset=part*n;
        let (mut a,mut b,mut aa,mut bb,mut ab)=(0.0,0.0,0.0,0.0,0.0);
        for i in 0..n {
            let x=audio[start+offset+i] as f64;let y=reference[offset+i] as f64;
            a+=x;b+=y;aa+=x*x;bb+=y*y;ab+=x*y;
        }
        let energy=((aa-a*a/n as f64)*(bb-b*b/n as f64)).max(0.0);
        let ok=energy>1e-20 && (ab-a*b/n as f64).abs()/energy.sqrt()>=0.10;
        if ok {matched+=1;}if part==3 {tail=ok;}
    }
    matched>=3 && tail
}

#[derive(Default)]
pub struct Detector {
    filter:Option<Filter>, initialized:bool, audio:VecDeque<f32>, pending:usize,
    total:usize, votes:[Option<(f64,usize)>;3],
}
impl Detector {
    pub fn push(&mut self, samples:&[f32], waveform:Option<&Waveform>, threshold:f32)->(Option<f32>,Option<Hit>) {
        if !self.initialized {
            self.filter=waveform.and_then(Filter::new);self.initialized=true;
        }
        let Some(filter)=&mut self.filter else {return (None,None);};
        let mut peak:Option<f32>=None;let mut hit=None;
        for (i,&sample) in samples.iter().enumerate() {
            self.audio.push_back(if sample.is_finite() {sample} else {0.0});
            if self.audio.len()>HISTORY {self.audio.pop_front();}
            self.pending+=1;
            self.total+=1;
            if self.audio.len()<LENGTH || self.pending<STEP {continue;}
            self.pending=0;
            for (index,(score,elapsed)) in filter.scan(&self.audio).into_iter().enumerate() {
                peak=Some(peak.map_or(score,|p|p.max(score)));
                if score<threshold {continue;}
                let completion=self.total as f64/RATE as f64-elapsed;
                // Distinct fragments must agree on the SAME completion time.
                // Repeated matches of one tone or a stale pre-silence vote do not count.
                let confirmed=self.votes.iter().enumerate().any(|(other,vote)|other!=index && vote.is_some_and(|(time,seen)|
                    self.total-seen<=RATE && (time-completion).abs()<=0.03));
                self.votes[index]=Some((completion,self.total));
                if confirmed && hit.is_none() {hit=Some(Hit {score,elapsed,end:i+1});}
            }
        }
        (peak,hit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn audio()->Vec<f32> {include_bytes!("../assets/spike-planted-reference-48k.pcm16").chunks_exact(2)
        .map(|b|i16::from_le_bytes([b[0],b[1]]) as f32/32768.0).collect()}
    fn random(n:usize)->Vec<f32> {
        let mut seed=71u32;(0..n).map(|_| {seed=seed.wrapping_mul(1664525).wrapping_add(1013904223);
            (seed as f64/u32::MAX as f64*2.0-1.0) as f32}).collect()
    }
    fn evaluate(signal:&[f32],reference:&Waveform)->(f32,Option<(usize,Hit)>) {
        let mut d=Detector::default();let mut best=0.0_f32;let mut first=None;
        for (i,p) in signal.chunks(137).enumerate() {
            let (score,hit)=d.push(p,Some(reference),0.25);
            if let Some(score)=score {best=best.max(score);}
            if first.is_none() {first=hit.map(|h|(i*137+h.end,h));}
        }
        (best,first)
    }
    #[test]
    fn waveform_survives_strong_additive_noise_without_spectral_shape_assumption() {
        let a=audio();let reference=Waveform::from_audio(&a,0.0);
        let n=random(a.len());let rms=(a.iter().map(|x|x*x).sum::<f32>()/a.len() as f32).sqrt();
        // Independent interference has twice the signal RMS (approximately -6 dB SNR).
        let mixed:Vec<_>=a.iter().zip(n).map(|(a,n)|a+n*rms*2.0*3.0_f32.sqrt()).collect();
        let (score,hit)=evaluate(&mixed,&reference);
        assert!(hit.is_some(),"NCC missed noisy fixture: {score}");assert!(score<=1.0);
    }
    #[test]
    fn local_normalization_matches_at_arbitrary_sample_lag_and_quiet_volume() {
        let a=audio();let reference=Waveform::from_audio(&a,0.0);
        let skip=RATE/4;let pad=173;
        let mut signal=vec![0.0;pad];signal.extend(a[skip..].iter().map(|x|x*0.001));signal.extend(vec![0.0;STEP]);
        let (score,hit)=evaluate(&signal,&reference);assert!(score>0.99,"{score}");
        let (end,hit)=hit.unwrap();let expected=(end+skip-pad) as f64/RATE as f64;
        assert!((hit.elapsed-expected).abs()<0.002,"{hit:?} expected {expected}");
    }
    #[test]
    fn silence_noise_reversed_and_preparation_do_not_trigger() {
        let a=audio();let reference=Waveform::from_audio(&a,0.0);
        let activation:Vec<_>=include_bytes!("../tests/fixtures/spike-activation-negative-48k.pcm16")
            .chunks_exact(2).map(|b|i16::from_le_bytes([b[0],b[1]]) as f32/32768.0).collect();
        let mut reversed=a;reversed.reverse();
        for (name,negative) in [("silence",vec![0.0;RATE]),("noise",random(RATE*2)),("reversed",reversed),("activation",activation)] {
            let (score,hit)=evaluate(&negative,&reference);assert!(hit.is_none(),"{name} false match: {score}");
        }
    }
    #[test]
    fn silence_is_not_removed_from_the_timeline() {
        let a=audio();let reference=Waveform::from_audio(&a,0.0);
        let mut broken=a[..RATE/4].to_vec();broken.extend(vec![0.0;RATE]);broken.extend(&a[RATE/4..]);
        let (_,hit)=evaluate(&broken,&reference);let (end,hit)=hit.unwrap();
        assert!(end>RATE);assert!((hit.elapsed-(end-RATE) as f64/RATE as f64).abs()<0.002);
    }
}
