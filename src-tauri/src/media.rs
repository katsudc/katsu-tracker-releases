//! Spotify transport via Windows SMTC; volume is Spotify's own audio sessions only.
use parking_lot::Mutex;
use serde::Serialize;
use std::sync::{Arc,mpsc};
use std::time::Duration;

#[derive(Clone,Default,Serialize)]
pub struct Snapshot {
    connected:bool,title:String,artist:String,album:String,cover:Option<String>,
    playing:bool,can_toggle:bool,can_next:bool,can_previous:bool,volume:Option<f32>,
    position_seconds:Option<f64>,duration_seconds:Option<f64>,
}
struct Command {action:String,volume:Option<f32>,reply:mpsc::SyncSender<Result<(),String>>}
pub struct Media {snapshot:Arc<Mutex<Snapshot>>,sender:mpsc::Sender<Command>}
fn spotify_source(source:&str)->bool {
    let s=source.to_ascii_lowercase();
    s=="spotify" || s=="spotify.exe" || s.ends_with("\\spotify.exe") || s.starts_with("spotifyab.spotifymusic_")
}
fn valid_volume(volume:Option<f32>)->Result<f32,String> {
    volume.filter(|v|v.is_finite() && (0.0..=1.0).contains(v)).ok_or("volume_invalid".into())
}
#[tauri::command]
pub fn media_status(state:tauri::State<'_,Media>)->Snapshot {state.snapshot.lock().clone()}
#[tauri::command]
pub async fn media_control(state:tauri::State<'_,Media>,action:String,volume:Option<f32>)->Result<(),String> {
    if !["toggle","next","previous","volume"].contains(&action.as_str()) {return Err("control_denied".into());}
    if action=="volume" {valid_volume(volume)?;}
    let sender=state.sender.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let (tx,rx)=mpsc::sync_channel(1);
        sender.send(Command {action,volume,reply:tx}).map_err(|_|"unavailable".to_string())?;
        rx.recv_timeout(Duration::from_secs(5)).map_err(|_|"unavailable".to_string())?
    }).await.map_err(|_|"unavailable".to_string())?
}

#[cfg(windows)]
mod native {
    use super::*;
    use base64::{engine::general_purpose::STANDARD,Engine};
    use windows::{core::Interface,Media::Control::*,Storage::Streams::DataReader};
    use windows::Win32::{Media::Audio::*,System::Com::{CoCreateInstance,CLSCTX_ALL}};
    pub fn session(manager:&GlobalSystemMediaTransportControlsSessionManager)->windows::core::Result<Option<GlobalSystemMediaTransportControlsSession>> {
        let sessions=manager.GetSessions()?;
        for i in 0..sessions.Size()? {
            let session=sessions.GetAt(i)?;
            if spotify_source(&session.SourceAppUserModelId()?.to_string()) {return Ok(Some(session));}
        }
        Ok(None)
    }
    fn cover(properties:&GlobalSystemMediaTransportControlsSessionMediaProperties)->Option<String> {
        let stream=properties.Thumbnail().ok()?.OpenReadAsync().ok()?.get().ok()?;
        let size=stream.Size().ok()?;
        if size==0 || size>512*1024 {return None;}
        let mime=stream.ContentType().ok()?.to_string();
        if !["image/jpeg","image/png","image/webp"].contains(&mime.as_str()) {return None;}
        let reader=DataReader::CreateDataReader(&stream.GetInputStreamAt(0).ok()?).ok()?;
        reader.LoadAsync(size as u32).ok()?.get().ok()?;
        let mut bytes=vec![0;size as usize];reader.ReadBytes(&mut bytes).ok()?;
        Some(format!("data:{mime};base64,{}",STANDARD.encode(bytes)))
    }
    fn volumes()->Vec<ISimpleAudioVolume> {
        let pids=crate::process::spotify_process_ids();if pids.is_empty(){return Vec::new();}
        let mut found=Vec::new();
        unsafe {
            let Ok(enumerator):Result<IMMDeviceEnumerator,_>=CoCreateInstance(&MMDeviceEnumerator,None,CLSCTX_ALL) else {return found;};
            let Ok(devices)=enumerator.EnumAudioEndpoints(eRender,DEVICE_STATE_ACTIVE) else {return found;};
            for i in 0..devices.GetCount().unwrap_or(0) {
                let Ok(device)=devices.Item(i) else {continue;};
                let Ok(manager):Result<IAudioSessionManager2,_>=device.Activate(CLSCTX_ALL,None) else {continue;};
                let Ok(sessions)=manager.GetSessionEnumerator() else {continue;};
                for j in 0..sessions.GetCount().unwrap_or(0) {
                    let Ok(control)=sessions.GetSession(j) else {continue;};
                    let Ok(info)=control.cast::<IAudioSessionControl2>() else {continue;};
                    if info.GetProcessId().is_ok_and(|pid|pids.contains(&pid)) {
                        if let Ok(volume)=control.cast::<ISimpleAudioVolume>() {found.push(volume);}
                    }
                }
            }
        }
        found
    }
    pub fn snapshot(session:&GlobalSystemMediaTransportControlsSession,old:&Snapshot)->windows::core::Result<Snapshot> {
        let properties=session.TryGetMediaPropertiesAsync()?.get()?;
        let title:String=properties.Title()?.to_string().chars().take(256).collect();
        let artist:String=properties.Artist()?.to_string().chars().take(256).collect();
        let album=properties.AlbumTitle()?.to_string();
        let cover=if old.title==title && old.artist==artist && old.album==album {old.cover.clone()} else {cover(&properties)};
        let playback=session.GetPlaybackInfo()?;let controls=playback.Controls()?;
        let volume=volumes().first().and_then(|v|unsafe {v.GetMasterVolume().ok()});
        let timeline=session.GetTimelineProperties().ok();
        let start=timeline.as_ref().and_then(|t|t.StartTime().ok()).map(|t|t.Duration).unwrap_or(0);
        let duration_seconds=timeline.as_ref().and_then(|t|t.EndTime().ok()).map(|t|(t.Duration-start) as f64/10_000_000.0).filter(|v|*v>0.0);
        let playing=playback.PlaybackStatus()?==GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing;
        let elapsed=if playing {
            let now=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs_f64()+11_644_473_600.0;
            timeline.as_ref().and_then(|t|t.LastUpdatedTime().ok()).map(|t|(now-t.UniversalTime as f64/10_000_000.0).clamp(0.0,86400.0)).unwrap_or(0.0)
        }else{0.0};
        let position_seconds=timeline.as_ref().and_then(|t|t.Position().ok()).map(|t|((t.Duration-start) as f64/10_000_000.0+elapsed).clamp(0.0,duration_seconds.unwrap_or(f64::MAX)));
        Ok(Snapshot {connected:true,title,artist,album,cover,position_seconds,duration_seconds,
            playing,
            can_toggle:controls.IsPlayPauseToggleEnabled().unwrap_or(false),can_next:controls.IsNextEnabled().unwrap_or(false),
            can_previous:controls.IsPreviousEnabled().unwrap_or(false),volume})
    }
    pub fn control(manager:&GlobalSystemMediaTransportControlsSessionManager,command:&Command)->Result<(),String> {
        let session=session(manager).map_err(|_|"unavailable")?.ok_or("no_session")?;
        if command.action=="volume" {
            let value=valid_volume(command.volume)?;
            let volumes=volumes();if volumes.is_empty(){return Err("volume_unavailable".into());}
            for volume in volumes {unsafe {volume.SetMasterVolume(value,std::ptr::null()).map_err(|_|"volume_unavailable")?;}}
            return Ok(());
        }
        let operation=match command.action.as_str() {
            "toggle"=>session.TryTogglePlayPauseAsync(),"next"=>session.TrySkipNextAsync(),"previous"=>session.TrySkipPreviousAsync(),_=>return Err("control_denied".into()),
        }.map_err(|_|"control_denied")?;
        if !operation.get().map_err(|_|"control_denied")? {return Err("control_denied".into());}
        Ok(())
    }
    pub fn worker(snapshot:Arc<Mutex<Snapshot>>,receiver:mpsc::Receiver<Command>) {
        use windows::Win32::System::WinRT::{RoInitialize,RoUninitialize,RO_INIT_MULTITHREADED};
        unsafe {if RoInitialize(RO_INIT_MULTITHREADED).is_err(){return;}}
        let mut manager=None;
        loop {
            if manager.is_none(){manager=GlobalSystemMediaTransportControlsSessionManager::RequestAsync().and_then(|op|op.get()).ok();}
            match receiver.recv_timeout(Duration::from_millis(900)) {
                Ok(command)=>{let result=manager.as_ref().ok_or("unavailable".into()).and_then(|m|control(m,&command));let _=command.reply.send(result);},
                Err(mpsc::RecvTimeoutError::Disconnected)=>break,
                Err(mpsc::RecvTimeoutError::Timeout)=>{},
            }
            if let Some(m)=manager.as_ref() {
                match session(m) {
                    Ok(Some(s))=> {let old=snapshot.lock().clone();if let Ok(next)=self::snapshot(&s,&old){*snapshot.lock()=next;}},
                    Ok(None)=>*snapshot.lock()=Snapshot::default(),
                    Err(_)=>{manager=None;*snapshot.lock()=Snapshot::default();},
                }
            }
        }
        unsafe {RoUninitialize();}
    }
}
pub fn setup(app:&tauri::AppHandle)->Result<(),Box<dyn std::error::Error>> {
    use tauri::Manager;
    let snapshot=Arc::new(Mutex::new(Snapshot::default()));let (tx,rx)=mpsc::channel();
    app.manage(Media {snapshot:snapshot.clone(),sender:tx});
    #[cfg(windows)] std::thread::Builder::new().name("spotify-media".into()).spawn(move ||native::worker(snapshot,rx))?;
    #[cfg(not(windows))] drop((snapshot,rx));
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn targets_spotify_not_browser_or_game_sessions() {
        for id in ["Spotify.exe","SpotifyAB.SpotifyMusic_zpdnekdrzrea0!Spotify","Spotify"] {assert!(spotify_source(id));}
        for id in ["chrome.exe","msedge.exe","VALORANT-Win64-Shipping.exe","FakeSpotify.exe"] {assert!(!spotify_source(id));}
    }
    #[test] fn rejects_nonfinite_and_out_of_range_volume() {
        for value in [f32::NAN,f32::INFINITY,-0.1,1.1] {assert!(valid_volume(Some(value)).is_err());}
        assert!(valid_volume(None).is_err());assert_eq!(valid_volume(Some(0.4)).unwrap(),0.4);
    }
    #[cfg(windows)]
    #[test] #[ignore="Reads the currently playing Spotify metadata and audio session; sends no controls"]
    fn live_spotify_read_only() {
        use windows::{Media::Control::GlobalSystemMediaTransportControlsSessionManager,Win32::System::WinRT::*};
        unsafe {RoInitialize(RO_INIT_MULTITHREADED).unwrap();}
        let manager=GlobalSystemMediaTransportControlsSessionManager::RequestAsync().unwrap().get().unwrap();
        let session=native::session(&manager).unwrap().expect("Start a song in Spotify for this check");
        let info=native::snapshot(&session,&Snapshot::default()).unwrap();
        assert!(info.connected);assert!(!info.title.is_empty());
        if let (Some(position),Some(duration))=(info.position_seconds,info.duration_seconds) {
            assert!(duration.is_finite()&&duration>0.0);assert!(position>=0.0&&position<=duration);
        }
        println!("Spotify metadata available; playing={}, cover={}, timeline={}, app_volume={:?}",info.playing,info.cover.is_some(),info.duration_seconds.is_some(),info.volume);
        unsafe {RoUninitialize();}
    }
    #[cfg(windows)]
    #[test] #[ignore="Briefly tests Spotify pause and app volume, then restores both"]
    fn live_spotify_controls_roundtrip() {
        use windows::{Media::Control::GlobalSystemMediaTransportControlsSessionManager,Win32::System::WinRT::*};
        unsafe {RoInitialize(RO_INIT_MULTITHREADED).unwrap();}
        fn execute(manager:&GlobalSystemMediaTransportControlsSessionManager,action:&str,volume:Option<f32>)->Result<(),String> {
            let (tx,_)=mpsc::sync_channel(1);
            native::control(manager,&Command {action:action.into(),volume,reply:tx})
        }
        struct Restore {manager:GlobalSystemMediaTransportControlsSessionManager,initial:Snapshot}
        impl Drop for Restore {
            fn drop(&mut self) {
                if let Some(v)=self.initial.volume {let _=execute(&self.manager,"volume",Some(v));}
                if let Ok(Some(session))=native::session(&self.manager) {
                    if let Ok(now)=native::snapshot(&session,&Snapshot::default()) {
                        if now.playing!=self.initial.playing {let _=execute(&self.manager,"toggle",None);}
                    }
                }
            }
        }
        {
            let manager=GlobalSystemMediaTransportControlsSessionManager::RequestAsync().unwrap().get().unwrap();
            let session=native::session(&manager).unwrap().expect("Open Spotify");
            let initial=native::snapshot(&session,&Snapshot::default()).unwrap();
            let restore=Restore {manager,initial};
            let volume=restore.initial.volume.expect("Spotify audio session must be active");
            let test_volume=(volume-0.05).max(0.0);
            execute(&restore.manager,"volume",Some(test_volume)).unwrap();
            let observed=native::snapshot(&session,&restore.initial).unwrap();
            assert!((observed.volume.unwrap()-test_volume).abs()<0.01);
            assert!(restore.initial.can_toggle);
            execute(&restore.manager,"toggle",None).unwrap();
            let mut changed=false;
            for _ in 0..10 {
                std::thread::sleep(Duration::from_millis(100));
                if native::snapshot(&session,&restore.initial).unwrap().playing!=restore.initial.playing {changed=true;break;}
            }
            assert!(changed,"Spotify playback did not change");
            drop(restore);
            std::thread::sleep(Duration::from_millis(200));
            let restored=native::snapshot(&session,&Snapshot::default()).unwrap();
            assert!((restored.volume.unwrap()-volume).abs()<0.01);
            println!("Spotify transport and per-app volume roundtrip passed; original settings restored");
        }
        unsafe {RoUninitialize();}
    }

}
