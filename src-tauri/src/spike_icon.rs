//! Read-only HUD recognition. Only the foreground game's top-center pixels are captured.
use std::time::{Duration, Instant};
const MASK: &[u8; 756] = include_bytes!("../assets/spike-icon.mask");
pub const THRESHOLD: f32 = 0.86;

pub fn similarity(pixels: &[u8], width: usize, height: usize, scale: f32) -> f32 {
    if pixels.len()!=width*height*4 || width<28 || height<27 {return 0.0;}
    if width!=160 || height!=96 {
        let mut small=vec![0;160*96*4];
        for y in 0..96 {for x in 0..160 {
            let src=((((2*y+1)*height/192).min(height-1))*width+((2*x+1)*width/320).min(width-1))*4;let dst=(y*160+x)*4;
            small[dst..dst+4].copy_from_slice(&pixels[src..src+4]);
        }}
        return similarity(&small,160,96,1.0);
    }
    let redness:Vec<f32>=pixels.chunks_exact(4).map(|p| {
        let (b,g,r)=(p[0] as f32,p[1] as f32,p[2] as f32);
        (r-g.max(b))/r.max(32.0)
    }).collect();
    let mut best=0.0_f32;
    for ratio in [0.70,0.80,0.90,1.0,1.1,1.2,1.35] {
        let w=(56.0*scale*ratio).round() as usize;
        let h=(54.0*scale*ratio).round() as usize;
        if w<20 || h<20 || w>width || h>height {continue;}
        for y in (0..=height-h).step_by(2) {
            for x in (0..=width-w).step_by(2) {
                let mut values=[0.0_f32;756];
                let (mut foreground,mut background)=(0,0);
                let (mut fg_sum,mut bg_sum)=(0.0,0.0);
                for ty in 0..27 {for tx in 0..28 {
                    let value=redness[(y+(ty*h/27).min(h-1))*width+x+(tx*w/28).min(w-1)];
                    let i=ty*28+tx;values[i]=value;
                    if MASK[i]!=0 {foreground+=1;fg_sum+=value;}
                    else {background+=1;bg_sum+=value;}
                }}
                let fg=fg_sum/foreground as f32;let bg=bg_sum/background as f32;
                // Compare the icon strokes with its own transparent holes, rather than
                // assigning the entire scene one fixed red threshold. Flat red walls
                // cannot pass this contrast gate. Keep the spatial shape checks below.
                if fg<0.35 || fg-bg<0.16 {continue;}
                let cutoff=(fg+bg)*0.5;
                let (mut yes,mut no)=(0,0);
                for (value,expected) in values.iter().zip(MASK) {
                    if *expected!=0 {yes+=usize::from(*value>cutoff);}
                    else {no+=usize::from(*value<=cutoff);}
                }
                let recall=yes as f32/foreground as f32;
                let specificity=no as f32/background as f32;
                if recall>=0.80 && specificity>=0.75 {best=best.max((recall+specificity)*0.5);}
            }
        }
    }
    best
}

/// Requires a witnessed absent -> present edge; never invent a plant timestamp on startup.
#[derive(Default)]
pub struct Gate { absent:Option<Instant>, first:Option<Instant>, last:Option<Instant>, armed:bool, hits:u8 }
impl Gate {
    pub fn reset(&mut self) {*self=Self::default();}
    pub fn update(&mut self, present:bool, now:Instant)->Option<Instant> {
        if self.last.is_some_and(|t|now.duration_since(t)>Duration::from_millis(350)) {self.reset();}
        self.last=Some(now);
        if !present {
            self.first=None;self.hits=0;
            let start=*self.absent.get_or_insert(now);
            if now.duration_since(start)>=Duration::from_millis(700) {self.armed=true;}
            return None;
        }
        self.absent=None;
        if !self.armed {return None;}
        let first=*self.first.get_or_insert(now);self.hits=self.hits.saturating_add(1);
        if self.hits>=3 {self.armed=false;self.first=None;self.hits=0;Some(first)} else {None}
    }
}

#[cfg(windows)]
pub struct Capture { pids:Vec<u32>, checked:Instant }
// Restore the caller's DPI context even when a capture returns early.
#[cfg(windows)]
struct DpiGuard(windows::Win32::UI::HiDpi::DPI_AWARENESS_CONTEXT);
#[cfg(windows)]
impl Drop for DpiGuard {
    fn drop(&mut self) { unsafe { windows::Win32::UI::HiDpi::SetThreadDpiAwarenessContext(self.0); } }
}
#[cfg(windows)]
impl Default for Capture {
    fn default()->Self {Self {pids:Vec::new(),checked:Instant::now()-Duration::from_secs(2)}}
}
#[cfg(windows)]
impl Capture {
    /// Returns None when the game is not foreground. No desktop/application images are retained.
    pub fn read(&mut self)->Result<Option<(Vec<u8>,usize,usize,f32,isize)>,String> {
        use windows::Win32::{Foundation::{RECT,POINT,HWND},Graphics::Gdi::*,UI::{WindowsAndMessaging::*,HiDpi::*}};
        if self.checked.elapsed()>=Duration::from_secs(1) {self.pids=crate::process::game_process_ids();self.checked=Instant::now();}
        unsafe {
            let previous=SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
            if previous.0.is_null() {return Err("Ekran koordinatları hazırlanamadı".into());}
            let _dpi=DpiGuard(previous);
            let hwnd=GetForegroundWindow();let mut pid=0;
            GetWindowThreadProcessId(hwnd,Some(&mut pid));
            if !self.pids.contains(&pid) || IsIconic(hwnd).as_bool() {return Ok(None);}
            let mut rect=RECT::default();GetClientRect(hwnd,&mut rect).map_err(|e|e.to_string())?;
            let (cw,ch)=(rect.right-rect.left,rect.bottom-rect.top);
            if cw<640 || ch<360 {return Ok(None);}
            let scale=ch as f32/1080.0;
            let (w,h)=((160.0*scale).round() as i32,(96.0*scale).round() as i32);
            if w<=0 || h<=0 || w>1024 || h>512 {return Err("Ekran boyutu desteklenmiyor".into());}
            // A DirectX window DC can expose a different surface (observed: bottom HUD).
            // Read the visible top HUD from the desktop, using physical client coordinates.
            let mut origin=POINT {x:(cw-w)/2,y:0};
            if !ClientToScreen(hwnd,&mut origin).as_bool() {return Err("Oyun konumu okunamadı".into());}
            let desktop=HWND::default();
            let dc=GetDC(desktop); if dc.0.is_null() {return Err("Oyun görüntüsü alınamadı".into());}
            let mem=CreateCompatibleDC(dc);let bmp=CreateCompatibleBitmap(dc,w,h);
            if mem.0.is_null() || bmp.0.is_null() {
                if !bmp.0.is_null() {let _=DeleteObject(bmp);}
                if !mem.0.is_null() {let _=DeleteDC(mem);}
                ReleaseDC(desktop,dc);return Err("Görüntü tamponu oluşturulamadı".into());
            }
            let old=SelectObject(mem,bmp);
            let copied=BitBlt(mem,0,0,w,h,dc,origin.x,origin.y,SRCCOPY);
            SelectObject(mem,old);
            let mut pixels=vec![0u8;(w*h*4) as usize];
            let mut info=BITMAPINFO::default();
            info.bmiHeader.biSize=std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            info.bmiHeader.biWidth=w;info.bmiHeader.biHeight=-h;
            info.bmiHeader.biPlanes=1;info.bmiHeader.biBitCount=32;info.bmiHeader.biCompression=BI_RGB.0;
            let lines=if copied.is_ok() {GetDIBits(dc,bmp,0,h as u32,Some(pixels.as_mut_ptr().cast()),&mut info,DIB_RGB_COLORS)} else {0};
            let _=DeleteObject(bmp);let _=DeleteDC(mem);ReleaseDC(desktop,dc);
            if GetForegroundWindow()!=hwnd {return Ok(None);}
            if lines!=h {return Err("Oyun görüntüsü okunamadı; kenarlıksız pencere kullan".into());}
            if pixels.chunks_exact(4).all(|p|p[0]<4 && p[1]<4 && p[2]<4) {return Err("Görüntü siyah; kenarlıksız pencere modunu kontrol et".into());}
            Ok(Some((pixels,w as usize,h as usize,scale,hwnd.0 as isize)))
        }
    }
}

#[cfg(test)]
mod tests {
 use super::*;
 #[cfg(windows)]
 #[test]
 #[ignore = "Requires the game foreground; saves only the small HUD region for manual verification"]
 fn native_hud_capture() {
  let mut capture=Capture::default();
  let deadline=Instant::now()+Duration::from_secs(20);
  while Instant::now()<deadline {
   if let Some((pixels,w,h,_,_))=capture.read().expect("native HUD capture") {
    let path=std::env::temp_dir().join("katsu-native-hud.bgra");
    std::fs::write(&path,&pixels).unwrap();
    println!("HUD {w}x{h}: {}",path.display());
    return;
   }
   std::thread::sleep(Duration::from_millis(200));
  }
  panic!("Game was not foreground; live capture was not verified");
 }
 #[test] fn actual_hud_icon_and_plain_colors() {
  let pixels=include_bytes!("../tests/fixtures/spike-icon.bgra");
  let score=similarity(pixels,160,96,1.0);assert!(score>=THRESHOLD,"{score}");
  for color in [[0,0,180,255],[30,90,120,255],[0,0,0,255]] {
   assert!(similarity(&color.repeat(160*96),160,96,1.0)<THRESHOLD);
  }
 }
 #[test] fn resized_hud_and_real_scene_negative() {
  let positive=include_bytes!("../tests/fixtures/spike-icon-1440.bgra");
  let score=similarity(positive,213,128,1440.0/1080.0);
  assert!(score>=THRESHOLD,"resized score {score}");
  assert!(similarity(include_bytes!("../tests/fixtures/spike-icon-negative.bgra"),160,96,1.0)<THRESHOLD);
 }
 #[test] fn live_icon_on_red_brick_background_is_detected() {
  let frame=include_bytes!("../tests/fixtures/spike-icon-live.bgra");
  assert!(similarity(frame,160,96,1.0)>=THRESHOLD);
  let mut dim=frame.to_vec();
  for pixel in dim.chunks_exact_mut(4) {for channel in &mut pixel[..3] {*channel=(*channel as f32*0.75) as u8;}}
  assert!(similarity(&dim,160,96,1.0)>=THRESHOLD);
  assert!(similarity(include_bytes!("../tests/fixtures/spike-icon-brick-negative.bgra"),160,96,1.0)<THRESHOLD);
 }
 #[test] fn translucent_icon_across_background_colors_and_wrong_shapes() {
  for background in [[150u8,70,70],[20,60,120],[180,180,180],[30,130,60],[15,15,15]] {
   let [r,g,b]=background;let mut image=[b,g,r,255].repeat(160*96);
   for y in 0..54 {for x in 0..56 {
    if MASK[(y/2)*28+x/2]!=0 {
     let i=((y+24)*160+x+52)*4;
     for (channel,ink) in [20.0,15.0,150.0].iter().enumerate() {
      image[i+channel]=(image[i+channel] as f32*0.25+ink*0.75) as u8;
     }
    }
   }}
   let score=similarity(&image,160,96,1.0);assert!(score>=THRESHOLD,"background {background:?}: {score}");
   // Same palette and area, but scrambled geometry, must not qualify as the icon.
   for (y,row) in image.chunks_exact_mut(160*4).enumerate() {row.rotate_left(((y*17)%160)*4);}
   assert!(similarity(&image,160,96,1.0)<THRESHOLD);
  }
 }
 #[test] fn startup_present_never_starts_then_single_edge_and_rearm() {
  let mut gate=Gate::default();let start=Instant::now();
  for i in 0..20 {assert!(gate.update(true,start+Duration::from_millis(i*100)).is_none());}
  for i in 20..30 {assert!(gate.update(false,start+Duration::from_millis(i*100)).is_none());}
  assert!(gate.update(true,start+Duration::from_millis(3000)).is_none());
  assert!(gate.update(true,start+Duration::from_millis(3100)).is_none());
  assert_eq!(gate.update(true,start+Duration::from_millis(3200)),Some(start+Duration::from_millis(3000)));
  for i in 33..50 {assert!(gate.update(true,start+Duration::from_millis(i*100)).is_none());}
 }
 #[test] fn focus_gap_and_one_frame_flash_do_not_trigger() {
  let mut gate=Gate::default();let start=Instant::now();
  for i in 0..10 {gate.update(false,start+Duration::from_millis(i*100));}
  assert!(gate.update(true,start+Duration::from_millis(1000)).is_none());
  gate.update(false,start+Duration::from_millis(1100));
  assert!(gate.update(true,start+Duration::from_millis(2000)).is_none());
  assert!(gate.update(true,start+Duration::from_millis(2100)).is_none());
  assert!(gate.update(true,start+Duration::from_millis(2200)).is_none());
 }
}
