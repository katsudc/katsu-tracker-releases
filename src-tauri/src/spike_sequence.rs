//! Independent subsequence matches: the beginning need not have been heard.
//! Preparation is diagnostic only; a countdown requires post-completion evidence.
use crate::spike_signal::{self as signal, Template, RATE, WINDOW, HOP, TEMPLATE_SECONDS};
use std::collections::VecDeque;

const FULL: usize = 50;
const PART: usize = 32; // 684 ms; never trigger from an isolated transient.
const PART_THRESHOLD: f32 = 0.86;

struct Candidate {
    frames: Vec<Vec<f32>>,
    elapsed: f64,
    terminal_from: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct Hit {
    pub score: f32,
    pub end: usize,
    /// Elapsed time since completion at the end of the matched audio window.
    pub elapsed: f64,
}

#[derive(Default)]
pub struct Observation {
    pub score: f32,
    pub preparing: bool,
    pub hit: Option<Hit>,
}

pub struct Detector {
    audio: VecDeque<f32>,
    frames: VecDeque<Vec<f32>>,
    until_hop: usize,
    candidates: Vec<Candidate>,
    legacy: signal::Detector,
}

impl Default for Detector {
    fn default() -> Self {
        Self { audio:VecDeque::with_capacity(WINDOW), frames:VecDeque::with_capacity(FULL),
            until_hop:WINDOW, candidates:vec![], legacy:Default::default() }
    }
}

impl Detector {
    fn prepare(&mut self, template: &Template) {
        // Always retain the original completion fingerprint, including old custom recordings.
        self.add_candidates(&template.frames, 0.0);
        if let Some(sequence) = &template.sequence {
            self.add_candidates(&sequence.frames, sequence.completion_seconds);
        }
    }

    fn add_candidates(&mut self, frames: &[Vec<f32>], completion: f64) {
        for length in [FULL, PART] {
            if frames.len() < length { continue; }
            let last = frames.len()-length;
            let mut starts: Vec<_> = (0..=last).step_by(8).collect();
            starts.push(last);
            starts.sort_unstable(); starts.dedup();
            for start in starts {
                let end = ((start+length-1)*HOP+WINDOW) as f64/RATE as f64;
                let terminal_from = (0..length).find(|i|
                    ((start+i)*HOP) as f64/RATE as f64 >= completion).unwrap_or(length);
                self.candidates.push(Candidate { frames:frames[start..start+length].to_vec(),
                    elapsed:end-completion, terminal_from });
            }
        }
    }

    pub fn push(&mut self, samples: &[f32], template: &Template, threshold: f32) -> Observation {
        let mut observation = Observation::default();
        if template.sample_rate == 8000 {
            if let Some((score,end)) = self.legacy.push(samples,template) {
                observation.score=score;
                if score >= threshold { observation.hit=Some(Hit {score,end,elapsed:TEMPLATE_SECONDS}); }
            }
            return observation;
        }
        if self.candidates.is_empty() { self.prepare(template); }
        for (position,&sample) in samples.iter().enumerate() {
            self.audio.push_back(if sample.is_finite() {sample} else {0.0});
            if self.audio.len()>WINDOW { self.audio.pop_front(); }
            self.until_hop-=1;
            if self.until_hop!=0 {continue;}
            self.until_hop=HOP;
            let audio:Vec<_>=self.audio.iter().copied().collect();
            let power=audio.iter().map(|x|x*x).sum::<f32>()/WINDOW as f32;
            self.frames.push_back(if power<1e-14 {vec![0.0;48]} else {signal::spectrum(&audio)});
            if self.frames.len()>FULL {self.frames.pop_front();}
            let mut best:Option<Hit>=None;
            for candidate in &self.candidates {
                if self.frames.len()<candidate.frames.len() {continue;}
                let (score,terminal)=score(&self.frames,candidate);
                observation.score=observation.score.max(score);
                let required=if candidate.frames.len()<FULL {threshold.max(PART_THRESHOLD)} else {threshold};
                if score<required {continue;}
                if candidate.elapsed<0.32 || candidate.frames.len()-candidate.terminal_from<10 {
                    observation.preparing=true;
                    continue;
                }
                if !terminal {continue;}
                if best.is_none_or(|hit|score>hit.score) {
                    best=Some(Hit {score,end:position+1,elapsed:candidate.elapsed});
                }
            }
            // Earliest qualified completion, not the loudest later sound in a packet.
            if observation.hit.is_none() { observation.hit=best; }
        }
        observation
    }
}

fn score(live:&VecDeque<Vec<f32>>, candidate:&Candidate)->(f32,bool) {
    let count=candidate.frames.len();
    let offset=live.len()-count;
    let mut values=[0.0_f32;FULL];
    for (i,reference) in candidate.frames.iter().enumerate() {
        values[i]=(i.saturating_sub(1)..=(i+1).min(count-1)).map(|j|
            live[offset+j].iter().zip(reference).map(|(a,b)|a*b).sum::<f32>())
            .fold(-1.0_f32,f32::max);
    }
    let values=&mut values[..count];
    // Do not let strong preparation evidence mask a cancelled/noisy ending.
    let tail=&values[candidate.terminal_from..];
    let terminal=tail.len()>=10 && tail.iter().sum::<f32>()/tail.len() as f32>=0.65
        && tail.iter().filter(|&&v|v>0.65).count()*10>=tail.len()*6;
    let mean=values.iter().sum::<f32>()/count as f32;
    let matching=values.iter().filter(|&&v|v>0.65).count();
    let regions=values.chunks(count.div_ceil(3)).filter(|part|
        part.iter().sum::<f32>()/part.len() as f32>0.65).count();
    if matching*10<count*6 || regions<2 {return (mean,terminal);}
    values.sort_by(f32::total_cmp);
    let trim=count*24/100;
    let robust=values[trim..].iter().sum::<f32>()/(count-trim) as f32;
    (robust.min(mean+0.12),terminal)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn audio()->Vec<f32> {include_bytes!("../assets/spike-planted-reference-48k.pcm16").chunks_exact(2)
        .map(|b|i16::from_le_bytes([b[0],b[1]]) as f32/32768.0).collect()}
    fn activation()->Vec<f32> {include_bytes!("../tests/fixtures/spike-activation-negative-48k.pcm16").chunks_exact(2)
        .map(|b|i16::from_le_bytes([b[0],b[1]]) as f32/32768.0).collect()}
    fn hits(samples:&[f32],template:&Template,threshold:f32,packet:usize)->Vec<(usize,Hit)> {
        let mut d=Detector::default();let mut out=vec![];
        for (i,p) in samples.chunks(packet).enumerate() {
            if let Some(h)=d.push(p,template,threshold).hit {out.push((i*packet+h.end,h));}
        }
        out
    }
    #[test]
    fn missing_beginning_still_detects_and_preserves_completion_clock() {
        let a=audio();let t=Template::from_audio(&a,0.0).unwrap();
        let skip=HOP*16;
        for packet in [137,960,4096] {
            let h=hits(&a[skip..],&t,0.82,packet);
            assert!(!h.is_empty(),"no suffix match, packet {packet}");
            let (end,hit)=h[0];
            assert!((hit.elapsed-(skip+end) as f64/RATE as f64).abs()<0.08,"{hit:?}");
        }
    }
    #[test]
    fn full_sequence_matches_without_hearing_its_start_and_cancel_does_not_trigger() {
        let mut full=activation();let completion=full.len() as f64/RATE as f64;
        full.extend(audio());let t=Template::from_full_audio(&full,0.0,completion).unwrap();
        assert!(t.valid());
        let encoded=serde_json::to_vec(&t).unwrap();
        let restored:Template=serde_json::from_slice(&encoded).unwrap();assert!(restored.valid());
        assert!(!hits(&full[RATE*2..],&restored,0.82,960).is_empty());
        let mut cancelled=activation();cancelled.extend(vec![0.0;RATE*2]);
        assert!(hits(&cancelled,&restored,0.82,960).is_empty(),"cancelled plant opened timer");
        let mut d=Detector::default();assert!(d.push(&activation(),&restored,0.82).preparing);
    }
    #[test]
    fn quiet_partial_with_loud_early_interference_and_negatives() {
        let a=audio();let t=Template::from_audio(&a,0.0).unwrap();
        let mut seed=173u32;
        let noise:Vec<f32>=(0..RATE*3).map(|_| {seed=seed.wrapping_mul(1664525).wrapping_add(1013904223);
            (seed as f64/u32::MAX as f64*2.0-1.0) as f32}).collect();
        let mut mixed:Vec<_>=a.iter().map(|x|x*0.001).collect();
        for i in 0..HOP*15 {mixed[i]+=noise[i]*0.5;}
        assert!(!hits(&mixed,&t,0.82,960).is_empty());
        let mut reversed=a.clone();reversed.reverse();
        for negative in [noise,activation(),reversed,vec![0.0;RATE*2]] {
            assert!(hits(&negative,&t,0.75,960).is_empty(),"negative triggered");
        }
    }
    #[test]
    fn invalid_markers_and_tampered_sequences_rejected() {
        let a=vec![0.1;RATE*12];
        for (start,end) in [(f64::NAN,2.0),(3.0,2.0),(0.0,9.0),(-1.0,2.0)] {
            assert!(Template::from_full_audio(&a,start,end).is_err());
        }
        let mut t=Template::from_audio(&audio(),0.0).unwrap();
        t.sequence=Some(signal::Sequence {frames:vec![],completion_seconds:1.0});assert!(!t.valid());
    }

    #[test]
    fn full_recording_uses_transition_when_late_completion_sound_is_obscured() {
        let mut full=activation();let completion=full.len();full.extend(audio());
        let template=Template::from_full_audio(&full,0.0,completion as f64/RATE as f64).unwrap();
        let mut seed=73u32;
        for value in &mut full[completion+RATE*6/10..] {
            seed=seed.wrapping_mul(1664525).wrapping_add(1013904223);
            *value+=(seed as f64/u32::MAX as f64*2.0-1.0) as f32*0.8;
        }
        let skipped=RATE*2;
        let found=hits(&full[skipped..],&template,0.82,960);
        assert!(!found.is_empty(),"clean transition should survive a masked late tail");
        let (end,hit)=found[0];
        let actual=(skipped+end) as f64/RATE as f64-completion as f64/RATE as f64;
        assert!(actual>=0.32,"must not start before completion evidence");
        assert!((actual-hit.elapsed).abs()<0.08,"clock must stay anchored: {actual} vs {hit:?}");
    }
}
