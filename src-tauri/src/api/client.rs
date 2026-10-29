use crate::api::types::*;
use serde::Deserialize;
use base64::{engine::general_purpose::STANDARD, Engine};
use flate2::read::DeflateDecoder;
use flate2::write::DeflateEncoder;
use flate2::Compression;
use parking_lot::RwLock;
use crate::party::{self, is_group_tag};
use futures_util::future::join_all;
use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Valorant not running")]
    NotRunning,
    #[error("Request failed: {0}")]
    RequestFailed(String),
    #[error("Parse error: {0}")]
    ParseError(String),
}

/// Lockfile data structure
#[derive(Clone, Debug)]
struct LockfileData {
    port: String,
    password: String,
    hash: String, // Simple hash of port:password for change detection
}

pub struct ValorantAPI {
    client: reqwest::Client, // Remote HTTPS: certificate validation enabled.
    local_client: reqwest::Client, // Loopback Riot API only; no proxy or redirects.
    pub puuid: RwLock<String>,
    pub region: RwLock<String>,
    pub shard: RwLock<String>,
    local_port: RwLock<String>,
    local_auth: RwLock<String>,
    remote_headers: RwLock<HashMap<String, String>>,
    pp_host: RwLock<String>, // Cloud player-preferences host (affinity-based, e.g. euc1.pp.sgp.pvp.net)
    pub connected: RwLock<bool>,
    pub needs_reinit: RwLock<bool>, // Signal that tokens need refresh
    last_init_time: RwLock<std::time::Instant>,
    refresh_after: RwLock<std::time::Duration>,
    initialize_lock: tokio::sync::Mutex<()>,
    rate_limit_until: RwLock<Option<std::time::Instant>>,
    last_session_warning: RwLock<Option<std::time::Instant>>,
    pub consecutive_network_errors: RwLock<u32>, // Track network errors for smarter disconnection
    lockfile_hash: RwLock<String>,               // Track lockfile content hash to detect changes
    last_lockfile_check: RwLock<std::time::Instant>, // Rate limit lockfile checks
    last_recovery_check: RwLock<std::time::Instant>, // Rate limit recovery checks
    /// Shared match-details cache (party detect + on-demand frequent-teammate scan).
    match_details_cache: RwLock<HashMap<String, MatchDetailsResponse>>,
    match_details_order: RwLock<std::collections::VecDeque<String>>,
    match_details_gate: tokio::sync::Mutex<Option<std::time::Instant>>,
    stats_read_gate: tokio::sync::Mutex<Option<std::time::Instant>>,
}

/// Outcome of a remote GET. Callers that care about match lifecycle must
/// distinguish a clean 404 (player left the match) from transient failures
/// (timeouts / 5xx / auth blips) so the UI does not flash "waiting".
#[derive(Debug)]
pub enum RemoteResult<T> {
    Ok(T),
    /// Resource genuinely absent (HTTP 404 / 204).
    NotFound,
    /// Network, auth, 5xx, parse, or other recoverable failure.
    Transient,
}

/// Probe of pregame/coregame player endpoints.
#[derive(Debug, Clone)]
pub enum MatchProbe {
    /// Player is registered in a match with this id.
    InMatch(String),
    /// Clean "not in this phase" (404 or empty MatchID).
    NotInMatch,
    /// Could not tell — keep last live snapshot.
    Uncertain,
}

fn nonempty_text(value: Option<String>) -> Option<String> {
    value.and_then(|s| {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

impl ValorantAPI {
    pub fn new() -> Self {
        let client = remote_http_client();
        let local_client = local_riot_http_client();

        Self {
            client,
            local_client,
            puuid: RwLock::new(String::new()),
            region: RwLock::new(String::new()),
            shard: RwLock::new(String::new()),
            local_port: RwLock::new(String::new()),
            local_auth: RwLock::new(String::new()),
            remote_headers: RwLock::new(HashMap::new()),
            pp_host: RwLock::new(String::new()),
            connected: RwLock::new(false),
            needs_reinit: RwLock::new(false),
            last_init_time: RwLock::new(std::time::Instant::now()),
            refresh_after: RwLock::new(std::time::Duration::from_secs(45*60)),
            initialize_lock: tokio::sync::Mutex::new(()),
            rate_limit_until: RwLock::new(None),
            last_session_warning: RwLock::new(None),
            consecutive_network_errors: RwLock::new(0),
            lockfile_hash: RwLock::new(String::new()),
            last_lockfile_check: RwLock::new(std::time::Instant::now()),
            last_recovery_check: RwLock::new(std::time::Instant::now()),
            match_details_cache: RwLock::new(HashMap::new()),
            match_details_order: RwLock::new(std::collections::VecDeque::new()),
            match_details_gate: tokio::sync::Mutex::new(None),
            stats_read_gate: tokio::sync::Mutex::new(None),
        }
    }

    /// Read lockfile and return parsed data
    fn read_lockfile() -> Result<LockfileData, ApiError> {
        let lockfile_path = format!(
            "{}\\Riot Games\\Riot Client\\Config\\lockfile",
            std::env::var("LOCALAPPDATA").unwrap_or_default()
        );

        let lockfile_content =
            std::fs::read_to_string(&lockfile_path).map_err(|_| ApiError::NotRunning)?;

        let parts: Vec<&str> = lockfile_content.split(':').collect();
        if parts.len() < 4 {
            return Err(ApiError::ParseError("Invalid lockfile".into()));
        }

        let port = parts[2].to_string();
        let password = parts[3].to_string();
        let hash = format!("{}:{}", port, password); // Simple hash for comparison

        Ok(LockfileData {
            port,
            password,
            hash,
        })
    }

    /// Check if lockfile has changed since last read (rate limited to once per second)
    pub fn check_lockfile_changed(&self) -> bool {
        let now = std::time::Instant::now();
        let last_check = *self.last_lockfile_check.read();

        // Rate limit to once per second
        if now.duration_since(last_check) < std::time::Duration::from_secs(1) {
            return false;
        }

        *self.last_lockfile_check.write() = now;

        if let Ok(lockfile) = Self::read_lockfile() {
            let current_hash = self.lockfile_hash.read().clone();
            if !current_hash.is_empty() && lockfile.hash != current_hash {
                tracing::debug!("[LockfileMonitor] Local Riot credentials changed");
                return true;
            }
        }
        false
    }

    /// Detect actual RiotClientServices port by checking which port the process is listening on
    /// This is a fallback when lockfile might be stale
    fn detect_riot_client_port() -> Option<String> {
        use std::os::windows::process::CommandExt;
        use std::process::Command;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        // First, find RiotClientServices PID
        // Use /NH to hide headers and /FO CSV for easier parsing
        let tasklist = Command::new("tasklist")
            .creation_flags(CREATE_NO_WINDOW)
            .args([
                "/FI",
                "IMAGENAME eq RiotClientServices.exe",
                "/FO",
                "CSV",
                "/NH",
            ])
            .output()
            .ok()?;

        let tasklist_output = String::from_utf8_lossy(&tasklist.stdout);

        // Parse PID from tasklist output (format: "RiotClientServices.exe","PID",...)
        let pid: Option<String> = tasklist_output.lines().next().and_then(|line| {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() >= 2 {
                Some(parts[1].trim_matches('"').to_string())
            } else {
                None
            }
        });

        let pid = pid?;
        tracing::debug!("[PortDetect] Found RiotClientServices PID: {}", pid);

        // Now find the port this PID is listening on (127.0.0.1)
        let netstat = Command::new("netstat")
            .creation_flags(CREATE_NO_WINDOW)
            .args(["-ano"])
            .output()
            .ok()?;

        let netstat_output = String::from_utf8_lossy(&netstat.stdout);

        // Look for LISTENING ports on 127.0.0.1 with matching PID
        for line in netstat_output.lines() {
            // Check for PID and Local Address 127.0.0.1
            // We check for "0.0.0.0:0" as Foreign Address to identify LISTENING sockets
            // This is safer than checking for word "LISTENING" which varies by system language
            if line.contains(&pid)
                && line.contains("127.0.0.1:")
                && (line.contains("LISTENING") || line.contains("0.0.0.0:0"))
            {
                // Parse port from line like: "  TCP    127.0.0.1:64011        0.0.0.0:0              LISTENING       2424"
                let parts: Vec<&str> = line.split_whitespace().collect();
                // Column 1: Proto, Column 2: Local Address
                if parts.len() >= 2 {
                    if let Some(port) = parts[1].split(':').last() {
                        tracing::debug!("[PortDetect] Found listening port: {}", port);
                        return Some(port.to_string());
                    }
                }
            }
        }

        tracing::debug!("[PortDetect] Could not find listening port for PID {}", pid);
        None
    }

    /// Try to recover connection by detecting actual Riot Client port
    /// Returns true if successfully updated credentials with new port
    pub fn try_recover_connection(&self) -> bool {
        // Rate limit recovery attempts (e.g., once every 3 seconds)
        let now = std::time::Instant::now();
        let last_check = *self.last_recovery_check.read();
        if now.duration_since(last_check) < std::time::Duration::from_secs(3) {
            return false;
        }
        *self.last_recovery_check.write() = now;

        tracing::debug!("[Recovery] Attempting to detect actual Riot Client port...");

        if let Some(detected_port) = Self::detect_riot_client_port() {
            let current_port = self.local_port.read().clone();

            if detected_port != current_port {
                tracing::debug!(
                    "[Recovery] Port mismatch! Current: {}, Detected: {}",
                    current_port,
                    detected_port
                );
                tracing::debug!("[Recovery] Will re-read lockfile to get new credentials...");

                // Re-read lockfile which should have the updated port
                if let Ok(lockfile) = Self::read_lockfile() {
                    if lockfile.port == detected_port {
                        tracing::debug!(
                            "[Recovery] Lockfile has correct port, updating credentials..."
                        );
                        let auth = STANDARD.encode(format!("riot:{}", lockfile.password));
                        *self.local_port.write() = lockfile.port.clone();
                        *self.local_auth.write() = format!("Basic {}", auth);
                        *self.lockfile_hash.write() = lockfile.hash;
                        *self.needs_reinit.write() = true;
                        return true;
                    } else {
                        // Lockfile is stale. We CANNOT force update because we don't know the new password.
                        // The password in the lockfile belongs to the OLD port.
                        tracing::warn!("[Recovery] Lockfile port ({}) matches old port, waiting for lockfile update...", lockfile.port);
                    }
                }
            } else {
                tracing::debug!(
                    "[Recovery] Port is correct ({}), issue might be temporary",
                    current_port
                );
            }
        }

        false
    }

    /// Refresh near the actual token expiry; opaque tokens use a bounded fallback.
    fn should_refresh_tokens(&self) -> bool {
        let last_init = *self.last_init_time.read();
        last_init.elapsed() >= *self.refresh_after.read()
    }

    pub fn retry_after_secs(&self)->Option<u32> {
        self.rate_limit_until.read().and_then(|until|until.checked_duration_since(std::time::Instant::now()))
            .map(|duration|duration.as_secs().saturating_add(1) as u32)
    }

    fn note_http_status(&self, status:u16, operation:&str) {
        if status==401 { *self.needs_reinit.write()=true; }
        if status==401 || status==403 || status==429 || status>=500 {
            let mut last=self.last_session_warning.write();
            if last.is_none_or(|at|at.elapsed()>=std::time::Duration::from_secs(15)) {
                tracing::warn!("[Session] operation={} HTTP={} refresh_requested={}",operation,status,status==401);
                *last=Some(std::time::Instant::now());
            }
        }
    }

    pub async fn initialize(&self) -> Result<ConnectionStatus, ApiError> {
        if crate::riot_offline_diagnostic() {
            return Err(ApiError::RequestFailed("Riot bağlantısı teşhis için kapalı".into()));
        }
        // Serialize manual and background requests: they must not initialize twice.
        let _initializing=self.initialize_lock.lock().await;
        let lockfile = Self::read_lockfile()?;
        let same_client=*self.lockfile_hash.read()==lockfile.hash;
        if same_client && *self.connected.read() {
            if !*self.needs_reinit.read() && !self.should_refresh_tokens() {
                return Ok(ConnectionStatus {connected:true,region:self.region.read().to_uppercase(),message:"Connected".into()});
            }
            if self.last_init_time.read().elapsed()<std::time::Duration::from_secs(15) {
                return Err(ApiError::RequestFailed("Riot oturum isteği kısa süreli bekletiliyor".into()));
            }
        }
        tracing::info!("[Session] Reading existing Riot session; new_client={}, requested={}, expired={}",!same_client,*self.needs_reinit.read(),self.should_refresh_tokens());

        let port = lockfile.port.clone();
        let password = lockfile.password.clone();
        let auth = STANDARD.encode(format!("riot:{}", password));

        *self.local_port.write() = port.clone();
        *self.local_auth.write() = format!("Basic {}", auth);
        *self.lockfile_hash.write() = lockfile.hash.clone(); // Save hash for change detection

        tracing::debug!(
            "[Initialize] Attempting connection to Riot Client on port {}",
            port
        );

        // Get entitlements with retry logic
        let ent_url = format!("https://127.0.0.1:{}/entitlements/v1/token", port);

        let mut last_error = String::new();
        let mut ent_response_opt: Option<EntitlementsResponse> = None;

        for attempt in 1..=3 {
            tracing::debug!(
                "[Initialize] Entitlements request attempt {}/3 to {}...",
                attempt,
                ent_url
            );

            match self
                .local_client
                .get(&ent_url)
                .header("Authorization", format!("Basic {}", auth))
                .send()
                .await
            {
                Ok(resp) => {
                    let status = resp.status();
                    if !status.is_success() {
                        last_error = format!("HTTP {}", status);
                        tracing::debug!("[Initialize] Request failed with status: {}", last_error);
                        if attempt < 3 {
                            tracing::debug!("[Initialize] Retrying in 4000ms...");
                            tokio::time::sleep(std::time::Duration::from_secs(4)).await;
                            continue;
                        }
                    } else {
                        match resp.json::<EntitlementsResponse>().await {
                            Ok(ent) => {
                                tracing::info!("[Initialize] Got entitlements successfully");
                                ent_response_opt = Some(ent);
                                break;
                            }
                            Err(e) => {
                                last_error = format!("JSON parse error: {}", e);
                                tracing::debug!("[Initialize] Parse error: {}", last_error);
                            }
                        }
                    }
                }
                Err(e) => {
                    last_error = e.to_string();
                    tracing::debug!(
                        "[Initialize] Connection error (attempt {}): {}",
                        attempt,
                        last_error
                    );

                    // Print the error source chain for debugging
                    use std::error::Error;
                    let mut source = e.source();
                    while let Some(s) = source {
                        tracing::debug!("[Initialize]   Caused by: {}", s);
                        source = s.source();
                    }

                    // Check specific error types
                    if e.is_connect() {
                        tracing::warn!(
                            "[Initialize]   -> Connection failed (check firewall/antivirus)"
                        );
                    }
                    if e.is_timeout() {
                        tracing::warn!("[Initialize]   -> Timeout occurred");
                    }

                    if attempt < 3 {
                        tracing::debug!("[Initialize] Retrying in 4000ms...");
                        tokio::time::sleep(std::time::Duration::from_secs(4)).await;
                    }
                }
            }
        }

        // Check if we succeeded in the loop above
        if ent_response_opt.is_none() {
            // All retries failed - try to detect actual port from process
            tracing::debug!("[Initialize] All connection attempts failed, trying port recovery...");
            if let Some(detected_port) = Self::detect_riot_client_port() {
                if detected_port != port {
                    tracing::debug!(
                        "[Initialize] Port mismatch detected! Lockfile: {}, Actual: {}",
                        port,
                        detected_port
                    );
                    tracing::debug!("[Initialize] Re-reading lockfile and retrying...");

                    // Re-read lockfile to get updated credentials
                    if let Ok(new_lockfile) = Self::read_lockfile() {
                        if new_lockfile.port == detected_port {
                            let new_auth =
                                STANDARD.encode(format!("riot:{}", new_lockfile.password));
                            *self.local_port.write() = new_lockfile.port.clone();
                            *self.local_auth.write() = format!("Basic {}", new_auth);
                            *self.lockfile_hash.write() = new_lockfile.hash;

                            // One more attempt with correct port
                            let new_url = format!(
                                "https://127.0.0.1:{}/entitlements/v1/token",
                                detected_port
                            );
                            tracing::debug!(
                                "[Initialize] Retrying with detected port: {}",
                                new_url
                            );

                            if let Ok(resp) = self
                                .local_client
                                .get(&new_url)
                                .header("Authorization", format!("Basic {}", new_auth))
                                .send()
                                .await
                            {
                                if resp.status().is_success() {
                                    if let Ok(ent) = resp.json::<EntitlementsResponse>().await {
                                        tracing::info!("[Initialize] Port recovery successful!");
                                        ent_response_opt = Some(ent);
                                    }
                                }
                            }
                        }
                    }
                } else {
                    tracing::debug!(
                        "[Initialize] Port matches detected port ({}), no recovery needed",
                        detected_port
                    );
                }
            }
        }

        let ent_response = ent_response_opt.ok_or_else(|| {
            tracing::error!(
                "[Initialize] All attempts failed. Last error: {}",
                last_error
            );
            ApiError::RequestFailed(last_error)
        })?;

        *self.puuid.write() = ent_response.subject.clone();

        // Get client version
        let version = self.get_client_version().await;

        let now=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
        let refresh_after=super::session::refresh_after(&ent_response.access_token,&ent_response.token,now);
        *self.refresh_after.write()=refresh_after;
        tracing::info!("[Session] Next token read in {} seconds",refresh_after.as_secs());

        // Set remote headers
        {
            let mut headers = self.remote_headers.write();
            headers.insert(
                "Authorization".into(),
                format!("Bearer {}", ent_response.access_token),
            );
            headers.insert("X-Riot-Entitlements-JWT".into(), ent_response.token);
            headers.insert("X-Riot-ClientPlatform".into(), "ew0KCSJwbGF0Zm9ybVR5cGUiOiAiUEMiLA0KCSJwbGF0Zm9ybU9TIjogIldpbmRvd3MiLA0KCSJwbGF0Zm9ybU9TVmVyc2lvbiI6ICIxMC4wLjE5MDQyLjEuMjU2LjY0Yml0IiwNCgkicGxhdGZvcm1DaGlwc2V0IjogIlVua25vd24iDQp9".into());
            headers.insert("X-Riot-ClientVersion".into(), version);
            headers.insert("Content-Type".into(), "application/json".into());
        }

        // Recovery may have changed the port/password; use the current pair.
        let port=self.local_port.read().clone();
        let auth=self.local_auth.read().clone();
        // Get region from sessions
        let sessions_url = format!(
            "https://127.0.0.1:{}/product-session/v1/external-sessions",
            port
        );
        if let Ok(resp) = self
            .local_client
            .get(&sessions_url)
            .header("Authorization", &auth)
            .send()
            .await
        {
            if let Ok(sessions) = resp.json::<HashMap<String, SessionData>>().await {
                for (_, session) in sessions {
                    if let Some(config) = session.launch_configuration {
                        if let Some(args) = config.arguments {
                            for arg in args {
                                if arg.contains("-ares-deployment=") {
                                    let region = arg.split('=').nth(1).unwrap_or("tr").to_string();
                                    *self.region.write() = region.clone();
                                }
                                if arg.contains("-config-endpoint=") {
                                    if let Some(endpoint) = arg.split('=').nth(1) {
                                        let parts: Vec<&str> = endpoint.split('.').collect();
                                        if parts.len() > 1 {
                                            *self.shard.write() = parts[1].to_string();
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Default region/shard
        if self.region.read().is_empty() {
            *self.region.write() = "tr".to_string();
        }
        if self.shard.read().is_empty() {
            *self.shard.write() = self.get_shard(&self.region.read());
        }

        // Settings affinity is resolved on demand. Basic tracking/autolock must
        // not request a separate PAS token on every session initialization.
        self.pp_host.write().clear();

        *self.connected.write() = true;
        *self.needs_reinit.write() = false;
        *self.consecutive_network_errors.write() = 0;
        *self.last_init_time.write() = std::time::Instant::now();

        Ok(ConnectionStatus {
            connected: true,
            region: self.region.read().to_uppercase(),
            message: "Connected".into(),
        })
    }

    fn get_shard(&self, region: &str) -> String {
        match region.to_lowercase().as_str() {
            "eu" | "tr" => "eu",
            "na" | "latam" | "br" => "na",
            "ap" => "ap",
            "kr" => "kr",
            _ => "eu",
        }
        .to_string()
    }

    /// Map an affinity / region code to the cloud player-preferences host prefix.
    /// The pp host uses the player-affinity prefix (e.g. `euc1`), which differs
    /// from the glz/pd shard. Verified live: EU -> euc1. Others are best-effort.
    fn pp_host_for_affinity(affinity: &str) -> String {
        let prefix = match affinity.to_lowercase().as_str() {
            "eu" | "eu1" | "euc1" | "tr" | "tr1" => "euc1",
            "na" | "na1" | "us" | "usw2" | "latam" | "br" => "usw2",
            "ap" | "ap1" | "apse1" | "sea" => "apse1",
            "kr" | "kr1" => "apse1", // KR routed via AP pp cluster (fallback)
            other if !other.is_empty() => return format!("{}.pp.sgp.pvp.net", other),
            _ => "prod",
        };
        format!("{}.pp.sgp.pvp.net", prefix)
    }

    async fn preferences_host(&self)->Result<String,String> {
        let cached=self.pp_host.read().clone();
        if !cached.is_empty() {return Ok(cached);}
        let bearer=self.remote_headers.read().get("Authorization").cloned().ok_or("Not connected")?;
        let token=bearer.strip_prefix("Bearer ").ok_or("Not connected")?;
        let host=self.resolve_pp_host(token).await;
        *self.pp_host.write()=host.clone();
        Ok(host)
    }

    /// Resolve the cloud player-preferences host. Prefers the PAS affinity token
    /// (authoritative), falls back to the local region table.
    async fn resolve_pp_host(&self, access_token: &str) -> String {
        // 1. PAS affinity token (authoritative). JWT: header.payload.signature,
        //    payload has an `affinity` claim like "eu1".
        if let Ok(resp) = self
            .client
            .get("https://riot-geo.pas.si.riotgames.com/pas/v1/product/valorant")
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await
        {
            if let Ok(jwt) = resp.text().await {
                if let Some(affinity) = Self::affinity_from_pas(&jwt) {
                    return Self::pp_host_for_affinity(&affinity);
                }
            }
        }
        // 2. Region table fallback.
        Self::pp_host_for_affinity(&self.region.read())
    }

    /// Extract the `affinity` claim from a PAS JWT (base64url payload).
    fn affinity_from_pas(jwt: &str) -> Option<String> {
        let payload_b64 = jwt.trim().split('.').nth(1)?;
        let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(payload_b64)
            .ok()?;
        let json: serde_json::Value = serde_json::from_slice(&decoded).ok()?;
        json.get("affinity")?.as_str().map(|s| s.to_string())
    }

    /// Raw DEFLATE inflate (no zlib/gzip wrapper). Matches Node `inflateRawSync`.
    fn inflate_raw(bytes: &[u8]) -> Result<Vec<u8>, String> {
        let mut out = Vec::new();
        DeflateDecoder::new(bytes)
            .read_to_end(&mut out)
            .map_err(|e| format!("inflate error: {}", e))?;
        Ok(out)
    }

    /// Raw DEFLATE compress (no zlib/gzip wrapper). Matches Node `deflateRawSync`.
    fn deflate_raw(bytes: &[u8]) -> Result<Vec<u8>, String> {
        let mut enc = DeflateEncoder::new(Vec::new(), Compression::default());
        enc.write_all(bytes)
            .map_err(|e| format!("deflate error: {}", e))?;
        enc.finish()
            .map_err(|e| format!("deflate finish error: {}", e))
    }

    /// Resolve the Riot client version used in remote PD/GLZ headers.
    ///
    /// **Order matters:** PD endpoints (especially `/mmr/v1/players/...`) return
    /// HTTP 404 `RESOURCE_NOT_FOUND` when `X-Riot-ClientVersion` does not match
    /// the locally installed build. `valorant-api.com` can lag a few shipping
    /// builds behind, so we prefer parsing `ShooterGame.log` first.
    async fn get_client_version(&self) -> String {
        if let Some(v) = Self::client_version_from_shooter_log() {
            tracing::info!("[Initialize] Client version from ShooterGame.log: {}", v);
            return v;
        }

        if let Ok(resp) = self
            .client
            .get("https://valorant-api.com/v1/version")
            .send()
            .await
        {
            if let Ok(data) = resp.json::<VersionResponse>().await {
                if let Some(d) = data.data {
                    if let Some(v) = d.riot_client_version {
                        tracing::warn!(
                            "[Initialize] Client version from valorant-api.com (log unavailable): {}",
                            v
                        );
                        return v;
                    }
                }
            }
        }

        tracing::warn!("[Initialize] Using hardcoded fallback client version");
        "release-13.04-shipping-18-5304478".to_string()
    }

    /// Parse `CI server version: release-XX.YY-shipping-N-NNNNNNN` from the
    /// local Valorant log (most accurate source while the game is running).
    fn client_version_from_shooter_log() -> Option<String> {
        let local = std::env::var("LOCALAPPDATA").ok()?;
        let log_path = PathBuf::from(local)
            .join("VALORANT")
            .join("Saved")
            .join("Logs")
            .join("ShooterGame.log");

        let content = std::fs::read_to_string(&log_path).ok()?;

        // Prefer the last "CI server version:" line — log is append-only across sessions.
        let mut found: Option<String> = None;
        for line in content.lines() {
            // Example: "CI server version: release-13.04-shipping-18-5304478"
            if let Some(idx) = line.find("CI server version:") {
                let rest = line[idx + "CI server version:".len()..].trim();
                let ver = rest
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if ver.starts_with("release-") {
                    found = Some(ver);
                }
            }
        }

        // Fallback: any release-*-shipping-*-* token near the end of the file
        if found.is_none() {
            for line in content.lines().rev().take(400) {
                for token in line.split_whitespace() {
                    if token.starts_with("release-") && token.contains("shipping") {
                        // Strip trailing punctuation if any
                        let clean = token.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '-' && c != '.');
                        if clean.starts_with("release-") {
                            return Some(clean.to_string());
                        }
                    }
                }
            }
        }

        found
    }

    fn glz_url(&self, endpoint: &str) -> String {
        let region = self.region.read();
        let shard = self.shard.read();
        let glz_region = if region.to_lowercase() == "tr" {
            "eu"
        } else {
            &region
        };
        format!(
            "https://glz-{}-1.{}.a.pvp.net{}",
            glz_region, shard, endpoint
        )
    }

    fn pd_url(&self, endpoint: &str) -> String {
        let shard = self.shard.read();
        format!("https://pd.{}.a.pvp.net{}", shard, endpoint)
    }

    async fn get_remote_ex<T: serde::de::DeserializeOwned>(&self, url: &str) -> RemoteResult<T> {
        // Check if tokens might be stale
        if self.should_refresh_tokens() {
            *self.needs_reinit.write() = true;
            // Don't immediately disconnect - let caller handle reinit
        }

        let headers: HashMap<String, String> = self.remote_headers.read().clone();

        // Retry mechanism - try up to 2 times
        for attempt in 1..=2 {
            let mut req = self.client.get(url);
            for (k, v) in headers.iter() {
                req = req.header(k, v);
            }

            match req.send().await {
                Ok(resp) => {
                    let status = resp.status().as_u16();
                    self.note_http_status(status,"remote_get");
                    if status==429 {
                        let seconds=resp.headers().get("Retry-After").and_then(|v|v.to_str().ok()).and_then(|v|v.parse::<u64>().ok()).unwrap_or(60).clamp(1,86400);
                        *self.rate_limit_until.write()=Some(std::time::Instant::now()+std::time::Duration::from_secs(seconds));
                    }
                    if status==403 {return RemoteResult::Transient;}
                    // Check for auth errors that indicate connection is stale
                    if status == 401 {
                        tracing::debug!("[get_remote] Auth error ({}), triggering reinit", status);
                        *self.needs_reinit.write() = true;
                        return RemoteResult::Transient;
                    }
                    // Clean absence — not an error, just "not in match / not found".
                    if status == 404 || status == 204 {
                        *self.consecutive_network_errors.write() = 0;
                        return RemoteResult::NotFound;
                    }
                    // Rate-limit / server errors: treat as transient, not match end.
                    if status == 429 || (500..600).contains(&status) {
                        tracing::debug!("[get_remote] Server/rate status {} (transient)", status);
                        let mut errors = self.consecutive_network_errors.write();
                        *errors += 1;
                        return RemoteResult::Transient;
                    }
                    if !(200..300).contains(&status) {
                        tracing::debug!("[get_remote] Unexpected status {} (transient)", status);
                        let mut errors = self.consecutive_network_errors.write();
                        *errors += 1;
                        return RemoteResult::Transient;
                    }
                    // Success body — parse JSON
                    match resp.json::<T>().await {
                        Ok(v) => {
                            *self.consecutive_network_errors.write() = 0;
                            return RemoteResult::Ok(v);
                        }
                        Err(e) => {
                            tracing::debug!("[get_remote] JSON parse failed: {}", e);
                            // 200 with unparseable body is not a clean "not in match".
                            return RemoteResult::Transient;
                        }
                    }
                }
                Err(e) => {
                    tracing::debug!("[get_remote] Request error (attempt {}): {}", attempt, e);

                    if attempt < 2 {
                        // Wait a bit before retry
                        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                        continue;
                    }

                    // After all retries failed, check if lockfile changed
                    // This is critical for detecting mid-game Riot Client restarts
                    if self.check_lockfile_changed() {
                        tracing::debug!(
                            "[get_remote] Lockfile changed during game! Triggering full reinit..."
                        );
                        *self.needs_reinit.write() = true;
                        *self.connected.write() = false;
                        return RemoteResult::Transient;
                    }

                    // If lockfile hasn't changed, try to detect actual port from process
                    // This handles cases where lockfile is stale
                    if self.try_recover_connection() {
                        tracing::debug!(
                            "[get_remote] Port recovery successful, triggering reinit..."
                        );
                        *self.connected.write() = false;
                        return RemoteResult::Transient;
                    }

                    // Increment error counter
                    let mut errors = self.consecutive_network_errors.write();
                    *errors += 1;
                    return RemoteResult::Transient;
                }
            }
        }
        RemoteResult::Transient
    }

    async fn get_remote<T: serde::de::DeserializeOwned>(&self, url: &str) -> Option<T> {
        match self.get_remote_ex(url).await {
            RemoteResult::Ok(v) => Some(v),
            RemoteResult::NotFound | RemoteResult::Transient => None,
        }
    }

    #[allow(dead_code)]
    async fn post_remote(&self, url: &str) -> Option<serde_json::Value> {
        // Check if tokens might be stale
        if self.should_refresh_tokens() {
            *self.needs_reinit.write() = true;
        }

        let headers: HashMap<String, String> = self.remote_headers.read().clone();

        // Retry mechanism - try up to 2 times
        for attempt in 1..=2 {
            let mut req = self.client.post(url);
            for (k, v) in headers.iter() {
                req = req.header(k, v);
            }

            match req.json(&serde_json::json!({})).send().await {
                Ok(resp) => {
                    self.note_http_status(resp.status().as_u16(),"remote_post");
                    if resp.status().as_u16() == 401 {
                        tracing::debug!(
                            "[post_remote] Auth error ({}), triggering reinit",
                            resp.status()
                        );
                        *self.needs_reinit.write() = true;
                        return None;
                    }
                    *self.consecutive_network_errors.write() = 0;
                    return resp.json().await.ok();
                }
                Err(e) => {
                    tracing::debug!("[post_remote] Request error (attempt {}): {}", attempt, e);

                    if attempt < 2 {
                        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                        continue;
                    }

                    // Check if lockfile changed (Riot Client might have restarted)
                    if self.check_lockfile_changed() {
                        tracing::debug!(
                            "[post_remote] Lockfile changed! Triggering full reinit..."
                        );
                        *self.needs_reinit.write() = true;
                        *self.connected.write() = false;
                        return None;
                    }

                    let mut errors = self.consecutive_network_errors.write();
                    *errors += 1;
                    return None;
                }
            }
        }
        None
    }

    /// Fire-and-forget POST - Errors do NOT affect connection state
    /// Used for agent select/lock where errors are non-critical
    async fn post_remote_silent(&self, url: &str) -> Option<serde_json::Value> {
        let headers: HashMap<String, String> = self.remote_headers.read().clone();
        let mut req = self.client.post(url);
        for (k, v) in headers.iter() {
            req = req.header(k, v);
        }
        match req.json(&serde_json::json!({})).send().await {
            Ok(resp) => {
                self.note_http_status(resp.status().as_u16(),"agent_select_or_lock");
                // An authentication failure requests a throttled token reread so the
                // next cycle refreshes them - otherwise select/lock would keep
                // silently failing and the agent would never get locked.
                if resp.status().as_u16() == 401 {
                    tracing::debug!(
                        "[post_remote_silent] Auth error ({}), triggering reinit",
                        resp.status()
                    );
                    *self.needs_reinit.write() = true;
                    return None;
                }
                if !resp.status().is_success() {return None;}
                resp.json().await.ok()
            }
            Err(e) => {
                tracing::error!("[post_remote_silent] Error (ignored): {}", e);
                None
            }
        }
    }

    pub async fn get_pregame_match_id(&self) -> Option<String> {
        match self.probe_pregame_match_id().await {
            MatchProbe::InMatch(id) => Some(id),
            MatchProbe::NotInMatch | MatchProbe::Uncertain => None,
        }
    }

    /// Probe pregame player endpoint distinguishing 404 vs transient failure.
    pub async fn probe_pregame_match_id(&self) -> MatchProbe {
        let puuid = self.puuid.read().clone();
        let url = self.glz_url(&format!("/pregame/v1/players/{}", puuid));
        match self.get_remote_ex::<PregamePlayer>(&url).await {
            RemoteResult::Ok(data) => match data.match_id.filter(|s| !s.is_empty()) {
                Some(id) => MatchProbe::InMatch(id),
                None => MatchProbe::NotInMatch,
            },
            RemoteResult::NotFound => MatchProbe::NotInMatch,
            RemoteResult::Transient => MatchProbe::Uncertain,
        }
    }

    pub async fn get_pregame_match(&self, match_id: &str) -> Option<PregameMatch> {
        match self.get_pregame_match_ex(match_id).await {
            RemoteResult::Ok(m) => Some(m),
            _ => None,
        }
    }

    pub async fn get_pregame_match_ex(&self, match_id: &str) -> RemoteResult<PregameMatch> {
        let url = self.glz_url(&format!("/pregame/v1/matches/{}", match_id));
        self.get_remote_ex(&url).await
    }

    pub async fn get_coregame_match_id(&self) -> Option<String> {
        match self.probe_coregame_match_id().await {
            MatchProbe::InMatch(id) => Some(id),
            MatchProbe::NotInMatch | MatchProbe::Uncertain => None,
        }
    }

    /// Probe coregame player endpoint distinguishing 404 vs transient failure.
    pub async fn probe_coregame_match_id(&self) -> MatchProbe {
        let puuid = self.puuid.read().clone();
        let url = self.glz_url(&format!("/core-game/v1/players/{}", puuid));
        match self.get_remote_ex::<CoregamePlayer>(&url).await {
            RemoteResult::Ok(data) => match data.match_id.filter(|s| !s.is_empty()) {
                Some(id) => MatchProbe::InMatch(id),
                None => MatchProbe::NotInMatch,
            },
            RemoteResult::NotFound => MatchProbe::NotInMatch,
            RemoteResult::Transient => MatchProbe::Uncertain,
        }
    }

    #[allow(dead_code)]
    pub async fn get_coregame_match(&self, match_id: &str) -> Option<CoregameMatch> {
        match self.get_coregame_match_ex(match_id).await {
            RemoteResult::Ok(m) => Some(m),
            _ => None,
        }
    }

    pub async fn get_coregame_match_ex(&self, match_id: &str) -> RemoteResult<CoregameMatch> {
        let url = self.glz_url(&format!("/core-game/v1/matches/{}", match_id));
        self.get_remote_ex(&url).await
    }

    pub async fn get_player_names(&self, puuids: &[String]) -> HashMap<String, String> {
        let url = self.pd_url("/name-service/v2/players");
        let headers: HashMap<String, String> = self.remote_headers.read().clone();
        let mut req = self.client.put(&url);
        for (k, v) in headers.iter() {
            req = req.header(k, v);
        }

        let mut names = HashMap::new();
        if let Ok(resp) = req.json(&puuids).send().await {
            if let Ok(data) = resp.json::<Vec<PlayerNameInfo>>().await {
                for p in data {
                    // Handle hidden/anonymous players - Riot returns empty game_name for privacy
                    // Return empty string for hidden players, caller will use agent name instead
                    let name = if p.game_name.is_empty() {
                        // Player has hidden their name - return empty, will be replaced with agent name
                        String::new()
                    } else if p.tag_line.is_empty() {
                        p.game_name
                    } else {
                        format!("{}#{}", p.game_name, p.tag_line)
                    };
                    names.insert(p.subject, name);
                }
            }
        }
        names
    }

    pub async fn select_agent(&self, match_id: &str, agent_id: &str) {
        let url = self.glz_url(&format!(
            "/pregame/v1/matches/{}/select/{}",
            match_id, agent_id
        ));
        let _ = self.post_remote_silent(&url).await;
    }

    pub async fn lock_agent(&self, match_id: &str, agent_id: &str) {
        let url = self.glz_url(&format!(
            "/pregame/v1/matches/{}/lock/{}",
            match_id, agent_id
        ));
        let _ = self.post_remote_silent(&url).await;
    }

    /// Cloud player-preferences GET. Returns the RAW settings JSON (serde Value)
    /// so nothing is lost on round-trip (keybind sub-fields, crosshair profiles,
    /// future keys). This is the authoritative source the game actually reads.
    ///
    ///   GET https://{pp_host}/playerPref/v3/getPreference/Ares.PlayerSettings
    ///   -> { type, data: base64(raw-deflate(JSON)), modified }
    pub async fn fetch_cloud_settings_raw(&self) -> Result<serde_json::Value, String> {
        let pp_host = self.preferences_host().await?;
        if pp_host.is_empty() {
            return Err("Not connected (no pp host)".to_string());
        }
        let url = format!(
            "https://{}/playerPref/v3/getPreference/Ares.PlayerSettings",
            pp_host
        );
        let headers = self.remote_headers.read().clone();
        let mut req = self.client.get(&url);
        for (k, v) in headers.iter() {
            req = req.header(k, v);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| format!("Cloud GET failed: {}", e))?;
        let status = resp.status().as_u16();
        if status == 401 {
            *self.needs_reinit.write() = true;
            return Err("STALE_TOKEN".to_string());
        }
        if !resp.status().is_success() {
            return Err(format!("Cloud GET HTTP {}", status));
        }
        let envelope: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Cloud GET parse error: {}", e))?;
        let data_b64 = envelope
            .get("data")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Cloud response missing 'data'".to_string())?;
        let compressed = STANDARD
            .decode(data_b64)
            .map_err(|e| format!("base64 decode error: {}", e))?;
        let json_bytes = Self::inflate_raw(&compressed)?;
        serde_json::from_slice(&json_bytes).map_err(|e| format!("settings JSON parse error: {}", e))
    }

    /// Cloud player-preferences PUT. Takes the RAW settings JSON (serde Value),
    /// compresses and uploads it. The game loads this on next launch.
    ///
    ///   PUT https://{pp_host}/playerPref/v3/savePreference
    ///   body { type:"Ares.PlayerSettings", data: base64(raw-deflate(JSON)) }
    pub async fn push_cloud_settings_raw(
        &self,
        settings: &serde_json::Value,
    ) -> Result<(), String> {
        if self.should_refresh_tokens() {
            *self.needs_reinit.write() = true;
            return Err("STALE_TOKEN".to_string());
        }
        let pp_host = self.preferences_host().await?;
        if pp_host.is_empty() {
            return Err("Not connected (no pp host)".to_string());
        }
        let json_bytes =
            serde_json::to_vec(settings).map_err(|e| format!("serialize error: {}", e))?;
        let compressed = Self::deflate_raw(&json_bytes)?;
        let data_b64 = STANDARD.encode(&compressed);
        let body = serde_json::json!({
            "type": "Ares.PlayerSettings",
            "data": data_b64,
        });

        let url = format!("https://{}/playerPref/v3/savePreference", pp_host);
        let headers = self.remote_headers.read().clone();
        let mut req = self.client.put(&url).json(&body);
        for (k, v) in headers.iter() {
            // json() sets Content-Type; don't double-insert it.
            if k.eq_ignore_ascii_case("content-type") {
                continue;
            }
            req = req.header(k, v);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| format!("Cloud PUT failed: {}", e))?;
        let status = resp.status().as_u16();
        if status == 401 {
            *self.needs_reinit.write() = true;
            return Err("STALE_TOKEN".to_string());
        }
        if !resp.status().is_success() {
            return Err(format!("Cloud PUT HTTP {}", status));
        }
        Ok(())
    }

    /// True for machine/account-specific settings that must NOT travel with a
    /// preset (audio device handles, monitor ids, onboarding flags). On apply we
    /// keep the TARGET account's own value for these instead of the preset's.
    fn is_machine_specific(setting_enum: &str) -> bool {
        const DROP_EXACT: &[&str] = &[
            "VoiceDeviceCaptureHandle",
            "VoiceDeviceRenderHandle",
            "DefaultMonitorDeviceID",
            "PreferredMonitorDeviceID",
            "PreferredGamePods",
        ];
        const DROP_PREFIX: &[&str] = &["HasSeen", "HasAccepted", "HasEver", "LastSeen"];
        let name = setting_enum.rsplit("::").next().unwrap_or(setting_enum);
        DROP_EXACT.contains(&name) || DROP_PREFIX.iter().any(|p| name.starts_with(p))
    }

    /// Build the blob to upload when applying a preset, using **full replace**
    /// semantics so the target ends up exactly like the preset:
    ///   - start from the preset (drop its machine-specific entries)
    ///   - carry over the TARGET account's machine-specific entries unchanged
    /// Settings absent from the preset are therefore absent from the upload, so
    /// Valorant resets them to default (e.g. a toggle that was at default when
    /// the preset was captured, then later changed, goes back to default).
    fn merge_for_apply(
        preset: &serde_json::Value,
        target_current: &serde_json::Value,
    ) -> serde_json::Value {
        let mut out = preset.clone();
        let Some(out_obj) = out.as_object_mut() else {
            return out;
        };

        for arr_key in [
            "boolSettings",
            "intSettings",
            "floatSettings",
            "stringSettings",
        ] {
            // Drop machine-specific entries that came from the preset's source.
            if let Some(arr) = out_obj.get_mut(arr_key).and_then(|v| v.as_array_mut()) {
                arr.retain(|item| {
                    item.get("settingEnum")
                        .and_then(|v| v.as_str())
                        .map(|s| !Self::is_machine_specific(s))
                        .unwrap_or(true)
                });
            }
            // Re-add the target account's own machine-specific entries.
            if let Some(target_arr) = target_current.get(arr_key).and_then(|v| v.as_array()) {
                let keep: Vec<serde_json::Value> = target_arr
                    .iter()
                    .filter(|item| {
                        item.get("settingEnum")
                            .and_then(|v| v.as_str())
                            .map(Self::is_machine_specific)
                            .unwrap_or(false)
                    })
                    .cloned()
                    .collect();
                if !keep.is_empty() {
                    let entry = out_obj
                        .entry(arr_key.to_string())
                        .or_insert_with(|| serde_json::Value::Array(Vec::new()));
                    if let Some(arr) = entry.as_array_mut() {
                        arr.extend(keep);
                    }
                }
            }
        }
        out
    }

    /// Apply a preset to the currently signed-in account's cloud, full-replace.
    /// Reads the target's current settings to preserve its machine-specific
    /// entries. Game must be closed (caller's responsibility); guards on token.
    pub async fn apply_player_settings(&self, settings: &serde_json::Value) -> Result<(), String> {
        // The target's current cloud settings — for machine-specific carry-over.
        let target_current = self
            .fetch_cloud_settings_raw()
            .await
            .unwrap_or_else(|_| serde_json::json!({}));
        let merged = Self::merge_for_apply(settings, &target_current);
        self.push_cloud_settings_raw(&merged).await
    }

    /// Typed view of the player's settings for UI/capture-meta. Prefers the cloud
    /// (authoritative); falls back to local files if the cloud is unreachable.
    pub async fn get_player_settings(&self) -> Result<PlayerSettingsResponse, String> {
        match self.fetch_cloud_settings_raw().await {
            Ok(raw) => {
                let data: PlayerSettingsData = serde_json::from_value(raw)
                    .map_err(|e| format!("settings typed parse error: {}", e))?;
                Ok(PlayerSettingsResponse {
                    data,
                    modified: 0,
                    settings_type: "Ares.PlayerSettings".to_string(),
                })
            }
            Err(e) => {
                tracing::debug!("[get_player_settings] cloud failed ({}), trying local", e);
                self.get_player_settings_local()
            }
        }
    }

    /// Read settings from the local plaintext config files (fallback only — the
    /// game ignores these on launch; cloud is authoritative).
    ///   %LOCALAPPDATA%\VALORANT\Saved\Config\<puuid>-<region>\
    ///     Windows\RiotUserSettings.ini      -> sensitivity / crosshair / audio
    ///     WindowsClient\BackupKeybinds.json  -> keybinds (actionMappings)
    pub fn get_player_settings_local(&self) -> Result<PlayerSettingsResponse, String> {
        let dir = self
            .resolve_valorant_config_dir()
            .ok_or_else(|| "Could not locate VALORANT config folder".to_string())?;

        let ini_path = dir.join("Windows").join("RiotUserSettings.ini");
        let ini = std::fs::read_to_string(&ini_path)
            .map_err(|e| format!("Failed to read {}: {}", ini_path.display(), e))?;

        let mut data = PlayerSettingsData {
            action_mappings: Vec::new(),
            bool_settings: Vec::new(),
            float_settings: Vec::new(),
            int_settings: Vec::new(),
            string_settings: Vec::new(),
            roaming_settings_version: 0,
        };

        for line in ini.lines() {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix("LocalSettingsVersion=") {
                data.roaming_settings_version = rest.trim().parse().unwrap_or(0);
                continue;
            }
            // Each setting line is "EAres<Kind>SettingName::Name=value".
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            if !key.starts_with("EAres") {
                continue;
            }
            let key = key.to_string();
            if key.starts_with("EAresBoolSettingName::") {
                data.bool_settings.push(BoolSetting {
                    setting_enum: key,
                    value: value.eq_ignore_ascii_case("true"),
                });
            } else if key.starts_with("EAresFloatSettingName::") {
                data.float_settings.push(FloatSetting {
                    setting_enum: key,
                    value: value.parse().unwrap_or(0.0),
                });
            } else if key.starts_with("EAresIntSettingName::") {
                data.int_settings.push(IntSetting {
                    setting_enum: key,
                    value: value.parse().unwrap_or(0),
                });
            } else if key.starts_with("EAresStringSettingName::") {
                data.string_settings.push(StringSetting {
                    setting_enum: key,
                    value: value.to_string(),
                });
            }
        }

        // Keybinds live in a separate JSON file.
        let keybinds_path = dir.join("WindowsClient").join("BackupKeybinds.json");
        if let Ok(raw) = std::fs::read_to_string(&keybinds_path) {
            if let Ok(kb) = serde_json::from_str::<KeybindsFile>(&raw) {
                data.action_mappings = kb.action_mappings;
            }
        }

        let modified = std::fs::metadata(&ini_path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        tracing::info!(
            "[get_player_settings] Read {} float / {} bool / {} int / {} string / {} keybinds from {}",
            data.float_settings.len(),
            data.bool_settings.len(),
            data.int_settings.len(),
            data.string_settings.len(),
            data.action_mappings.len(),
            ini_path.display()
        );

        Ok(PlayerSettingsResponse {
            data,
            modified,
            settings_type: "Ares.PlayerSettings".to_string(),
        })
    }

    /// Locate the active account's VALORANT config folder. Prefers the folder
    /// matching the current puuid; otherwise falls back to the most recently
    /// modified `<id>-<region>` folder that has a RiotUserSettings.ini.
    fn resolve_valorant_config_dir(&self) -> Option<PathBuf> {
        let base = PathBuf::from(std::env::var("LOCALAPPDATA").ok()?)
            .join("VALORANT")
            .join("Saved")
            .join("Config");

        let puuid = self.puuid.read().clone();
        let entries = std::fs::read_dir(&base).ok()?;

        let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            let ini = path.join("Windows").join("RiotUserSettings.ini");
            if !ini.exists() {
                continue;
            }
            // Exact puuid match wins immediately.
            if !puuid.is_empty() && name.starts_with(&puuid) {
                return Some(path);
            }
            let mtime = std::fs::metadata(&ini)
                .and_then(|m| m.modified())
                .unwrap_or(std::time::UNIX_EPOCH);
            if best.as_ref().map_or(true, |(t, _)| mtime > *t) {
                best = Some((mtime, path));
            }
        }
        best.map(|(_, p)| p)
    }

    /// Get presences from local chat API - returns puuid -> party_id map
    pub async fn get_presences(&self) -> HashMap<String, String> {
        let port = self.local_port.read().clone();
        let auth = self.local_auth.read().clone();
        let url = format!("https://127.0.0.1:{}/chat/v4/presences", port);

        let mut party_map = HashMap::new();

        match self
            .local_client
            .get(&url)
            .header("Authorization", &auth)
            .send()
            .await
        {
            Ok(resp) => {
                // Check for auth errors
                if resp.status().as_u16() == 401 {
                    tracing::debug!(
                        "[get_presences] Auth error ({}), checking lockfile...",
                        resp.status()
                    );
                    // Check if lockfile changed (port/password might have changed)
                    if self.check_lockfile_changed() {
                        tracing::debug!("[get_presences] Lockfile changed, triggering reinit");
                    }
                    *self.needs_reinit.write() = true;
                    return party_map;
                }

                if let Ok(data) = resp.json::<PresencesResponse>().await {
                    for p in data.presences {
                        if let Some(private_b64) = p.private {
                            if let Ok(decoded) = STANDARD.decode(&private_b64) {
                                if let Ok(json_str) = String::from_utf8(decoded) {
                                    if let Ok(private_data) =
                                        serde_json::from_str::<PresencePrivate>(&json_str)
                                    {
                                        if let Some(party_id) = private_data.party_id {
                                            if !party_id.is_empty() {
                                                party_map.insert(p.puuid, party_id);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                // Local API unreachable - check if lockfile changed (Riot Client might have restarted)
                tracing::debug!("[get_presences] Connection error: {}", e);

                // Check if lockfile changed - this is the KEY check for the mid-game issue
                if self.check_lockfile_changed() {
                    tracing::debug!("[get_presences] Lockfile changed! Port may have changed, triggering full reinit...");
                    *self.needs_reinit.write() = true;
                    *self.connected.write() = false;
                    return party_map;
                }

                // Increment error counter
                let mut errors = self.consecutive_network_errors.write();
                *errors += 1;
            }
        }
        party_map
    }

    /// Local Riot chat session. Works in menus, not only inside a match.
    pub async fn get_my_riot_id(&self) -> Option<(String, String)> {
        let port = self.local_port.read().clone();
        let auth = self.local_auth.read().clone();
        if port.is_empty() || auth.is_empty() {
            return None;
        }
        let url = format!("https://127.0.0.1:{port}/chat/v1/session");
        let resp = self
            .local_client
            .get(&url)
            .header("Authorization", &auth)
            .send()
            .await
            .ok()?;
        if !resp.status().is_success() {
            return None;
        }
        #[derive(Deserialize)]
        struct ChatSession {
            #[serde(default)]
            puuid: String,
            #[serde(default, alias = "gameName")]
            game_name: String,
            #[serde(default, alias = "gameTag")]
            game_tag: String,
        }
        let session = resp.json::<ChatSession>().await.ok()?;
        let name = session.game_name.trim();
        if name.is_empty() {
            return None;
        }
        let tag = session.game_tag.trim();
        let display = if tag.is_empty() {
            name.to_string()
        } else {
            format!("{name}#{tag}")
        };
        Some((session.puuid, display))
    }

    /// Read our OWN presence: session loop phase + live round score.
    ///
    /// The GLZ match endpoints do not expose the score; the Riot client
    /// publishes it in private presence as partyOwnerMatchScoreAllyTeam/EnemyTeam.
    ///
    /// After a match ends the client *may* flip `sessionLoopState` to `MENUS`
    /// and reset scores to 0-0, but it often stays `INGAME` through the
    /// post-game scoreboard. Callers must not treat leftover INGAME presence
    /// as proof the match is still live.
    pub async fn get_my_presence(&self) -> Option<MyPresence> {
        let port = self.local_port.read().clone();
        let auth = self.local_auth.read().clone();
        let my_puuid = self.puuid.read().clone();
        let url = format!("https://127.0.0.1:{}/chat/v4/presences", port);

        let resp = match self
            .local_client
            .get(&url)
            .header("Authorization", &auth)
            .send()
            .await
        {
            Ok(r) => r,
            Err(_) => return None,
        };

        if let Ok(data) = resp.json::<PresencesResponse>().await {
            for p in data.presences {
                if p.puuid != my_puuid {
                    continue;
                }
                if let Some(private_b64) = p.private {
                    if let Ok(decoded) = STANDARD.decode(&private_b64) {
                        if let Ok(json_str) = String::from_utf8(decoded) {
                            if let Ok(pd) = serde_json::from_str::<PresencePrivate>(&json_str) {
                                // Scores are only meaningful while actually INGAME.
                                // In MENUS the client resets them to 0-0 — returning
                                // those would make Discord show a fake "Map 0-0".
                                let ingame = pd
                                    .session_loop_state
                                    .as_deref()
                                    .is_some_and(|s| s.eq_ignore_ascii_case("INGAME"));
                                let flow = nonempty_text(pd.provisioning_flow)
                                    .or_else(|| nonempty_text(pd.party_owner_provisioning_flow));
                                return Some(MyPresence {
                                    session_loop_state: pd.session_loop_state,
                                    ally_score: if ingame {
                                        pd.party_owner_match_score_ally_team
                                    } else {
                                        None
                                    },
                                    enemy_score: if ingame {
                                        pd.party_owner_match_score_enemy_team
                                    } else {
                                        None
                                    },
                                    queue_id: nonempty_text(pd.queue_id),
                                    party_state: nonempty_text(pd.party_state),
                                    party_size: pd.party_size.filter(|n| *n > 0),
                                    provisioning_flow: flow,
                                });
                            }
                        }
                    }
                }
                break;
            }
        }
        None
    }

    /// Get my party info - returns (party_id, member_puuids)
    pub async fn get_my_party(&self) -> (Option<String>, Vec<String>) {
        let puuid = self.puuid.read().clone();
        let url = self.glz_url(&format!("/parties/v1/players/{}", puuid));

        if let Some(data) = self.get_remote::<PartyPlayerResponse>(&url).await {
            if let Some(party_id) = data.current_party_id {
                let party_url = self.glz_url(&format!("/parties/v1/parties/{}", party_id));
                if let Some(party_data) = self.get_remote::<PartyResponse>(&party_url).await {
                    let members: Vec<String> = party_data
                        .members
                        .iter()
                        .filter_map(|m| m.subject.clone())
                        .collect();
                    return (Some(party_id), members);
                }
                return (Some(party_id), vec![puuid]);
            }
        }
        (None, vec![])
    }

    /// Detect parties for a list of players (legacy - kept for compatibility)
    #[allow(dead_code)]
    pub async fn detect_parties(&self, puuids: &[String]) -> HashMap<String, String> {
        let mut party_map: HashMap<String, String> = HashMap::new();
        let mut party_counter: HashMap<String, u32> = HashMap::new();
        let mut next_party_num: u32 = 1;
        let mut found_via_presence: std::collections::HashSet<String> =
            std::collections::HashSet::new();

        // Get my party info
        let (my_party_id, my_party_members) = self.get_my_party().await;

        // Get presences for friends
        let presences = self.get_presences().await;

        for puuid in puuids {
            // Check if in my party
            if let Some(ref my_pid) = my_party_id {
                if my_party_members.contains(puuid) {
                    if !party_counter.contains_key(my_pid) {
                        party_counter.insert(my_pid.clone(), next_party_num);
                        next_party_num += 1;
                    }
                    party_map.insert(puuid.clone(), format!("Grup-{}", party_counter[my_pid]));
                    found_via_presence.insert(puuid.clone());
                    continue;
                }
            }

            // Check presences (friends)
            if let Some(friend_party_id) = presences.get(puuid) {
                if !party_counter.contains_key(friend_party_id) {
                    party_counter.insert(friend_party_id.clone(), next_party_num);
                    next_party_num += 1;
                }
                party_map.insert(
                    puuid.clone(),
                    format!("Grup-{}", party_counter[friend_party_id]),
                );
                found_via_presence.insert(puuid.clone());
            }
        }

        // For players not found via presence, try match history method
        let unknown_puuids: Vec<String> = puuids
            .iter()
            .filter(|p| !found_via_presence.contains(*p))
            .cloned()
            .collect();

        if !unknown_puuids.is_empty() {
            // Try to get party info from match history
            let history_parties = self.detect_parties_via_history(&unknown_puuids).await;

            for (puuid, party_tag) in history_parties {
                if !party_map.contains_key(&puuid) {
                    // Renumber the groups to continue from where we left off
                    if party_tag.starts_with("Grup-") {
                        let new_tag = format!("Grup-{}", next_party_num);
                        // Check if this is a new group we haven't seen
                        let existing_count =
                            party_map.values().filter(|v| *v == &party_tag).count();
                        if existing_count == 0 {
                            party_map.insert(puuid, new_tag);
                            next_party_num += 1;
                        } else {
                            party_map.insert(puuid, party_tag);
                        }
                    } else {
                        party_map.insert(puuid, party_tag);
                    }
                }
            }
        }

        // Mark remaining as Solo
        for puuid in puuids {
            if !party_map.contains_key(puuid) {
                party_map.insert(puuid.clone(), "Solo".into());
            }
        }

        // Filter single-person groups
        let mut party_sizes: HashMap<String, u32> = HashMap::new();
        for tag in party_map.values() {
            *party_sizes.entry(tag.clone()).or_insert(0) += 1;
        }

        for (_puuid, tag) in party_map.iter_mut() {
            if party_sizes.get(tag).copied().unwrap_or(0) == 1 {
                *tag = "Solo".into();
            }
        }

        party_map
    }

    /// Get player current competitive tier + RR.
    ///
    /// Returns `None` when the MMR request failed (network/auth/parse) so callers
    /// can retry later. Returns `Some((0, 0))` when the player is confirmed
    /// unranked / has no competitive data.
    ///
    /// Use the explicit current update, its referenced season, or the current
    /// competitive field. Historical act performance is never used to guess rank.
    pub async fn get_player_mmr(&self, puuid: &str) -> Option<(u32, u32)> {
        let url = self.pd_url(&format!("/mmr/v1/players/{}", puuid));
        // Distinguish "not found / transport error" from a successful empty rank.
        let data = match self.get_remote_ex::<MmrResponse>(&url).await {
            RemoteResult::Ok(data) => data,
            RemoteResult::NotFound => {
                // PUUID has no MMR resource — treat as confirmed unranked.
                return Some((0, 0));
            }
            RemoteResult::Transient => {
                tracing::debug!("[get_player_mmr] transient failure for {}", puuid);
                return None;
            }
        };

        crate::rank::current_rank(&data)
    }

    /// Get player peak rank across all competitive seasons
    /// Returns (peak_tier, peak_rank_name, peak_rank_color, season_id)
    pub async fn get_player_peak_rank(&self, puuid: &str) -> Option<(u32, String, String, String)> {
        use crate::constants::{BEFORE_ASCENDANT_SEASONS, RANK_NAMES};

        let url = self.pd_url(&format!("/mmr/v1/players/{}", puuid));
        let data: MmrResponse = self.get_remote(&url).await?;

        let queue_skills = data.queue_skills?;
        let competitive = queue_skills.competitive?;
        let seasons = competitive.seasonal_info_by_season_id?;

        let mut max_tier: u32 = 0;
        let mut max_tier_season = String::new();

        for (season_id, season_info) in seasons {
            // Check WinsByTier - keys are tier numbers as strings
            if let Some(ref wins_by_tier) = season_info.wins_by_tier {
                for (tier_str, _wins) in wins_by_tier {
                    if let Ok(mut tier) = tier_str.parse::<u32>() {
                        // Apply Ascendant offset for old seasons
                        // Before Ascendant, Immortal was tier 21-23, Radiant was 24
                        // After Ascendant, Ascendant is 21-23, Immortal is 24-26, Radiant is 27
                        if BEFORE_ASCENDANT_SEASONS.contains(&season_id.as_str()) && tier > 20 {
                            tier += 3; // Shift old Immortal/Radiant to new tier numbers
                        }

                        if tier > max_tier {
                            max_tier = tier;
                            max_tier_season = season_id.clone();
                        }
                    }
                }
            }

            // Also check CompetitiveTier / Rank directly
            let tier = season_info.effective_tier();
            if tier > 0 {
                let mut adjusted_tier = tier;
                if BEFORE_ASCENDANT_SEASONS.contains(&season_id.as_str()) && tier > 20 {
                    adjusted_tier += 3;
                }
                if adjusted_tier > max_tier {
                    max_tier = adjusted_tier;
                    max_tier_season = season_id.clone();
                }
            }
        }

        if max_tier == 0 {
            return None;
        }

        let (rank_name, rank_color) = RANK_NAMES
            .get(&max_tier)
            .map(|(n, c)| (n.to_string(), c.to_string()))
            .unwrap_or_else(|| ("Unknown".to_string(), "#768079".to_string()));

        Some((max_tier, rank_name, rank_color, max_tier_season))
    }

    /// Get match history for a player (last N matches)
    pub async fn get_match_history(&self, puuid: &str, count: u32) -> Vec<String> {
        self.get_match_history_opt(puuid, count)
            .await
            .unwrap_or_default()
    }

    // Stats metadata and detail requests share a modest budget, separate from live game polling.
    async fn wait_for_stats_read(&self)->bool {
        let mut last=self.stats_read_gate.lock().await;
        if self.retry_after_secs().is_some(){return false;}
        if let Some(started)=*last {
            if let Some(wait)=std::time::Duration::from_millis(500).checked_sub(started.elapsed()) {tokio::time::sleep(wait).await;}
        }
        if self.retry_after_secs().is_some(){return false;}
        *last=Some(std::time::Instant::now());true
    }

    /// Last competitive match ids. `None` means the history request failed.
    /// Entries with a non-competitive `QueueID` are dropped in case the query
    /// filter is ignored.
    pub async fn get_competitive_match_ids(&self, puuid: &str, take: usize) -> Option<Vec<String>> {
        if puuid != self.puuid.read().as_str() {return None;}
        if !self.wait_for_stats_read().await{return None;}
        const SCAN: u32 = 20;
        let url = self.pd_url(&format!(
            "/match-history/v1/history/{}?startIndex=0&endIndex={}&queue=competitive",
            puuid, SCAN
        ));
        match self.get_remote_ex::<MatchHistoryResponse>(&url).await {
            RemoteResult::Ok(data) => Some(
                data.history
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|h| match h.queue_id.as_deref() {
                        Some(q) => q.eq_ignore_ascii_case("competitive"),
                        None => true,
                    })
                    .map(|h| h.match_id)
                    .take(take)
                    .collect(),
            ),
            _ => None,
        }
    }

    /// Own-profile current rank and peak tier from one session MMR read.
    pub async fn get_rank_snapshot(
        &self,
        puuid: &str,
    ) -> Option<crate::rank::RankSnapshot> {
        if !self.wait_for_stats_read().await{return None;}
        let url = self.pd_url(&format!("/mmr/v1/players/{}", puuid));
        match self.get_remote_ex::<MmrResponse>(&url).await {
            RemoteResult::Ok(data) => Some(crate::rank::rank_snapshot_from_mmr(&data)),
            RemoteResult::NotFound => Some(crate::rank::RankSnapshot {current_tier:Some(0),current_rr:Some(0),..Default::default()}),
            RemoteResult::Transient => None,
        }
    }

    /// Like [`get_match_history`], but `None` means the request failed (retry).
    /// `Some(vec)` is a successful parse — empty when the player has no games.
    pub async fn get_match_history_opt(&self, puuid: &str, count: u32) -> Option<Vec<String>> {
        if puuid != self.puuid.read().as_str() {return None;}
        let url = self.pd_url(&format!(
            "/match-history/v1/history/{}?startIndex=0&endIndex={}",
            puuid, count
        ));

        match self.get_remote_ex::<MatchHistoryResponse>(&url).await {
            RemoteResult::Ok(data) => Some(
                data.history
                    .unwrap_or_default()
                    .into_iter()
                    .map(|h| h.match_id)
                    .collect(),
            ),
            _ => None,
        }
    }

    pub fn cached_match_details(&self,match_id:&str)->Option<MatchDetailsResponse> {
        self.match_details_cache.read().get(match_id).cloned()
    }

    /// Get match details (contains partyId for all players). Cached in-process
    /// so party detection and frequent-teammate scans share the same bodies.
    pub async fn get_match_details(&self, match_id: &str) -> Option<MatchDetailsResponse> {
        const CACHE_CAP: usize = 256;
        if match_id.is_empty() {
            return None;
        }
        if let Some(cached) = self.match_details_cache.read().get(match_id).cloned() {
            return Some(cached);
        }
        // One shared gate across roster stats, party detection and manual scans.
        // Cache hits bypass it; queued duplicates recheck after acquiring it.
        let mut last=self.match_details_gate.lock().await;
        if let Some(cached)=self.cached_match_details(match_id) {return Some(cached);}
        if self.retry_after_secs().is_some() {return None;}
        if let Some(started)=*last {
            let spacing=std::time::Duration::from_millis(750);
            if let Some(wait)=spacing.checked_sub(started.elapsed()) {tokio::time::sleep(wait).await;}
        }
        if self.retry_after_secs().is_some() {return None;}
        if !self.wait_for_stats_read().await{return None;}
        *last=Some(std::time::Instant::now());
        let url = self.pd_url(&format!("/match-details/v1/matches/{}", match_id));
        let data: MatchDetailsResponse = self.get_remote(&url).await?;
        {
            let mut cache = self.match_details_cache.write();
            let mut order = self.match_details_order.write();
            if !cache.contains_key(match_id) {
                if cache.len() >= CACHE_CAP {
                    if let Some(old) = order.pop_front() {
                        cache.remove(&old);
                    }
                }
                cache.insert(match_id.to_string(), data.clone());
                order.push_back(match_id.to_string());
            }
        }
        Some(data)
    }

    /// Detect parties using match history - checks recent matches for party groupings
    #[allow(dead_code)]
    pub async fn detect_parties_via_history(&self, puuids: &[String]) -> HashMap<String, String> {
        let mut party_map: HashMap<String, String> = HashMap::new();
        let mut party_counter: HashMap<String, u32> = HashMap::new();
        let mut next_party_num: u32 = 1;

        // We need to find a common recent match to get party info
        // Strategy: Get last 3 matches of first player for better coverage

        if puuids.is_empty() {
            return party_map;
        }

        // Get last 3 matches of first player
        let first_puuid = &puuids[0];
        let match_ids = self.get_match_history(first_puuid, 3).await;

        // Collect party info from both matches
        let mut all_match_parties: HashMap<String, String> = HashMap::new();

        for match_id in &match_ids {
            if let Some(details) = self.get_match_details(match_id).await {
                if let Some(players) = details.players {
                    for p in players {
                        // Only add if not already found (prefer more recent match)
                        if !all_match_parties.contains_key(&p.subject) {
                            all_match_parties.insert(p.subject.clone(), p.party_id.clone());
                        }
                    }
                }
            }
        }

        // Map party IDs to group numbers for target puuids
        for puuid in puuids {
            if let Some(party_id) = all_match_parties.get(puuid) {
                if !party_id.is_empty() {
                    if !party_counter.contains_key(party_id) {
                        party_counter.insert(party_id.clone(), next_party_num);
                        next_party_num += 1;
                    }
                    party_map.insert(puuid.clone(), format!("Grup-{}", party_counter[party_id]));
                } else {
                    party_map.insert(puuid.clone(), "Solo".into());
                }
            }
        }

        // For any puuids not found in the matches, mark as solo
        for puuid in puuids {
            if !party_map.contains_key(puuid) {
                party_map.insert(puuid.clone(), "Solo".into());
            }
        }

        // Filter single-person groups (they're actually solo)
        let mut party_sizes: HashMap<String, u32> = HashMap::new();
        for tag in party_map.values() {
            *party_sizes.entry(tag.clone()).or_insert(0) += 1;
        }

        for (_puuid, tag) in party_map.iter_mut() {
            if tag.starts_with("Grup-") && party_sizes.get(tag).copied().unwrap_or(0) == 1 {
                *tag = "Solo".into();
            }
        }

        party_map
    }

    /// Get current game loadouts for all players
    pub async fn get_coregame_loadouts(&self, match_id: &str) -> Option<LoadoutsResponse> {
        let url = self.glz_url(&format!("/core-game/v1/matches/{}/loadouts", match_id));
        self.get_remote(&url).await
    }

    /// Get pregame loadouts for all players
    pub async fn get_pregame_loadouts(&self, match_id: &str) -> Option<PregameLoadoutsResponse> {
        let url = self.glz_url(&format!("/pregame/v1/matches/{}/loadouts", match_id));
        self.get_remote(&url).await
    }

    /// Fetch the logged-in user's storefront (daily shop + optional night market).
    /// Riot only returns the authenticated user's own store, so we use self puuid.
    /// store/v3 requires POST with an empty JSON body (v2 GET was removed).
    pub async fn get_storefront(&self) -> Option<StorefrontResponse> {
        let puuid = self.puuid.read().clone();
        if puuid.is_empty() {
            tracing::warn!("[get_storefront] empty puuid");
            return None;
        }
        let url = self.pd_url(&format!("/store/v3/storefront/{}", puuid));
        let headers: HashMap<String, String> = self.remote_headers.read().clone();

        let mut req = self.client.post(&url);
        for (k, v) in headers.iter() {
            req = req.header(k, v);
        }

        match req.json(&serde_json::json!({})).send().await {
            Ok(resp) => {
                let status = resp.status();
                self.note_http_status(status.as_u16(),"storefront");
                if status.as_u16() == 401 {
                    tracing::warn!(
                        "[get_storefront] auth error ({}), triggering reinit",
                        status
                    );
                    *self.needs_reinit.write() = true;
                    return None;
                }
                let body = resp.text().await.unwrap_or_default();
                if !status.is_success() {
                    tracing::warn!(
                        "[get_storefront] HTTP {} body: {}",
                        status,
                        &body.chars().take(300).collect::<String>()
                    );
                    return None;
                }
                match serde_json::from_str::<StorefrontResponse>(&body) {
                    Ok(data) => Some(data),
                    Err(e) => {
                        tracing::warn!(
                            "[get_storefront] parse error: {} | body: {}",
                            e,
                            &body.chars().take(500).collect::<String>()
                        );
                        None
                    }
                }
            }
            Err(e) => {
                tracing::warn!("[get_storefront] request error: {}", e);
                None
            }
        }
    }

    /// Fetch the logged-in user's currency balances (VP / Radianite / Kingdom).
    pub async fn get_wallet(&self) -> Option<WalletResponse> {
        let puuid = self.puuid.read().clone();
        if puuid.is_empty() {
            return None;
        }
        let url = self.pd_url(&format!("/store/v1/wallet/{}", puuid));
        self.get_remote(&url).await
    }

    /// Detect parties with player-level caching.
    ///
    /// Confirmed `Grup-*` tags in `existing_cache` are kept. Solo placeholders
    /// are re-checked so a late presence or a full history sweep can upgrade them.
    ///
    /// History is fetched for **every** player in `players_to_fetch` (not just
    /// the first) — otherwise a solo first player hides stacks among the rest.
    pub async fn detect_parties_with_cache(
        &self,
        all_puuids: &[String],
        players_to_fetch: &[String],
        existing_cache: &HashMap<String, String>,
        team_of: &HashMap<String, String>,
    ) -> PartyDetectResult {
        const HISTORY_LOOKBACK: u32 = 5;
        const HISTORY_CONCURRENCY: usize = 4;
        const MAX_DETAIL_MATCHES: usize = 8;

        let mut party_map: HashMap<String, String> = HashMap::new();
        let mut party_id_to_num: HashMap<String, u32> = HashMap::new();
        let mut history_fetched: HashSet<String> = HashSet::new();

        let mut next_party_num = party::next_group_num(existing_cache);

        // Step 1: Keep only confirmed groups. Solo is a placeholder, not a fact.
        for (puuid, party) in existing_cache {
            if is_group_tag(party) {
                party_map.insert(puuid.clone(), party.clone());
            }
        }

        // Step 2: Own party + friend presences (live, accurate when available)
        let (my_party_id, my_party_members) = self.get_my_party().await;
        let presences = self.get_presences().await;

        for puuid in all_puuids {
            if party_map.get(puuid).is_some_and(|t| is_group_tag(t)) {
                continue;
            }

            if let Some(ref my_pid) = my_party_id {
                if my_party_members.contains(puuid) {
                    if !party_id_to_num.contains_key(my_pid) {
                        party_id_to_num.insert(my_pid.clone(), next_party_num);
                        next_party_num += 1;
                    }
                    party_map.insert(puuid.clone(), format!("Grup-{}", party_id_to_num[my_pid]));
                    continue;
                }
            }

            if let Some(friend_party_id) = presences.get(puuid) {
                if !party_id_to_num.contains_key(friend_party_id) {
                    party_id_to_num.insert(friend_party_id.clone(), next_party_num);
                    next_party_num += 1;
                }
                party_map.insert(
                    puuid.clone(),
                    format!("Grup-{}", party_id_to_num[friend_party_id]),
                );
            }
        }

        // Step 3: Recent-match sweep for everyone still unknown
        let own_id=self.puuid.read().clone();
        let need_history: Vec<String> = players_to_fetch
            .iter()
            .filter(|p|p.as_str()==own_id)
            .filter(|p| !party_map.get(*p).is_some_and(|t| is_group_tag(t)))
            .cloned()
            .collect();

        if !need_history.is_empty() {
            let lobby: HashSet<&str> = all_puuids.iter().map(|s| s.as_str()).collect();
            let mut histories: HashMap<String, Vec<String>> = HashMap::new();

            let mut remaining = need_history.clone();
            while !remaining.is_empty() {
                let n = HISTORY_CONCURRENCY.min(remaining.len());
                let chunk: Vec<String> = remaining.drain(..n).collect();
                let futs: Vec<_> = chunk
                    .iter()
                    .map(|puuid| {
                        let p = puuid.clone();
                        async move {
                            let hist = self.get_match_history_opt(&p, HISTORY_LOOKBACK).await;
                            (p, hist)
                        }
                    })
                    .collect();
                for (puuid, hist) in join_all(futs).await {
                    if let Some(ids) = hist {
                        history_fetched.insert(puuid.clone());
                        histories.insert(puuid, ids);
                    }
                }
            }

            // Prefer matches that appear in several lobby histories, then
            // more recent games, capped so we don't fetch 30 match-details.
            let mut candidates: Vec<String> = Vec::new();
            let mut seen_matches: HashSet<String> = HashSet::new();
            for depth in 0..HISTORY_LOOKBACK as usize {
                for ids in histories.values() {
                    if let Some(id) = ids.get(depth) {
                        if seen_matches.insert(id.clone()) {
                            candidates.push(id.clone());
                        }
                    }
                }
                if candidates.len() >= MAX_DETAIL_MATCHES {
                    break;
                }
            }
            candidates.truncate(MAX_DETAIL_MATCHES);

            let mut appearances: Vec<party::PartyAppearance> = Vec::new();
            for match_id in &candidates {
                if let Some(details) = self.get_match_details(match_id).await {
                    if let Some(players) = details.players {
                        for p in players {
                            if lobby.contains(p.subject.as_str()) && !p.party_id.is_empty() {
                                appearances.push((
                                    p.subject,
                                    match_id.clone(),
                                    p.party_id,
                                ));
                            }
                        }
                    }
                }
            }

            let clusters = party::cluster_historical_parties(&appearances, team_of);
            let mut cluster_to_tag: HashMap<u32, String> = HashMap::new();
            for (puuid, cluster_id) in &clusters {
                if let Some(existing) = party_map.get(puuid).filter(|t| is_group_tag(t)) {
                    cluster_to_tag
                        .entry(*cluster_id)
                        .or_insert_with(|| existing.clone());
                }
            }
            for (puuid, cluster_id) in clusters {
                if party_map.get(&puuid).is_some_and(|t| is_group_tag(t)) {
                    continue;
                }
                let tag = cluster_to_tag.entry(cluster_id).or_insert_with(|| {
                    let t = format!("Grup-{}", next_party_num);
                    next_party_num += 1;
                    t
                });
                party_map.insert(puuid, tag.clone());
            }

            tracing::info!(
                "[parties] history sweep: fetched={}/{} matches={} clustered={}",
                history_fetched.len(),
                need_history.len(),
                candidates.len(),
                party_map.values().filter(|t| is_group_tag(t)).count()
            );
        }

        // Step 4: Remaining players are Solo for this pass
        for puuid in all_puuids {
            if !party_map.contains_key(puuid) {
                party_map.insert(puuid.clone(), "Solo".into());
            }
        }

        // Step 5: Collapse 1-person Grup-* tags (keep previously confirmed ones)
        let mut party_sizes: HashMap<String, u32> = HashMap::new();
        for tag in party_map.values() {
            *party_sizes.entry(tag.clone()).or_insert(0) += 1;
        }

        for (puuid, tag) in party_map.iter_mut() {
            if existing_cache.get(puuid).is_some_and(|t| is_group_tag(t)) {
                continue;
            }
            if is_group_tag(tag) && party_sizes.get(tag).copied().unwrap_or(0) == 1 {
                *tag = "Solo".into();
            }
        }

        PartyDetectResult {
            parties: party_map,
            history_fetched,
        }
    }
}

/// Result of a live party-detection pass.
pub struct PartyDetectResult {
    pub parties: HashMap<String, String>,
    /// Players whose match-history request succeeded (empty list counts).
    pub history_fetched: HashSet<String>,
}



fn parse_conversations_json(raw: &str) -> Option<ConversationsResponse> {
    if let Ok(parsed) = serde_json::from_str::<ConversationsResponse>(raw) {
        return Some(parsed);
    }
    if let Ok(list) = serde_json::from_str::<Vec<Conversation>>(raw) {
        return Some(ConversationsResponse { conversations: list });
    }
    if let Ok(one) = serde_json::from_str::<Conversation>(raw) {
        if !one.cid.is_empty() {
            return Some(ConversationsResponse {
                conversations: vec![one],
            });
        }
    }
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(raw) {
        let arr = value
            .get("conversations")
            .or_else(|| value.get("Conversations"))
            .cloned()
            .or_else(|| value.as_array().cloned().map(serde_json::Value::Array));
        if let Some(arr) = arr {
            let mut conversations = Vec::new();
            if let Some(items) = arr.as_array() {
                for item in items {
                    if let Ok(conv) = serde_json::from_value::<Conversation>(item.clone()) {
                        if !conv.cid.is_empty() {
                            conversations.push(conv);
                        }
                    }
                }
            }
            return Some(ConversationsResponse { conversations });
        }
    }
    tracing::warn!(
        "[conversations] parse failed ({} bytes): {}",
        raw.len(),
        raw.chars().take(240).collect::<String>()
    );
    None
}

fn parse_chat_history_json(raw: &str) -> Option<ChatHistoryResponse> {
    if let Ok(parsed) = serde_json::from_str::<ChatHistoryResponse>(raw) {
        return Some(parsed);
    }
    if let Ok(list) = serde_json::from_str::<Vec<ChatMessage>>(raw) {
        return Some(ChatHistoryResponse { messages: list });
    }
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(raw) {
        let arr = value
            .get("messages")
            .or_else(|| value.get("Messages"))
            .cloned()
            .or_else(|| value.as_array().cloned().map(serde_json::Value::Array));
        if let Some(arr) = arr {
            let mut messages = Vec::new();
            if let Some(items) = arr.as_array() {
                for item in items {
                    if let Ok(msg) = serde_json::from_value::<ChatMessage>(item.clone()) {
                        messages.push(msg);
                    }
                }
            }
            return Some(ChatHistoryResponse { messages });
        }
    }
    tracing::warn!(
        "[chat_history] parse failed ({} bytes): {}",
        raw.len(),
        raw.chars().take(240).collect::<String>()
    );
    None
}

impl ValorantAPI {
    // Chat API Methods

    /// Get all active conversations
    pub async fn get_conversations(&self) -> Option<ConversationsResponse> {
        let port = self.local_port.read().clone();
        let auth = self.local_auth.read().clone();
        let url = format!("https://127.0.0.1:{}/chat/v6/conversations", port);

        match self
            .local_client
            .get(&url)
            .header("Authorization", &auth)
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                parse_conversations_json(&resp.text().await.unwrap_or_default())
            }
            Ok(resp) => {
                tracing::debug!("[get_conversations] HTTP {}", resp.status());
                None
            }
            Err(_) => None,
        }
    }

    /// Get chat history for all or specific conversation
    pub async fn get_chat_history(&self, cid: Option<&str>) -> Option<ChatHistoryResponse> {
        let port = self.local_port.read().clone();
        let auth = self.local_auth.read().clone();

        let url = if let Some(conversation_id) = cid {
            format!(
                "https://127.0.0.1:{}/chat/v6/messages?cid={}",
                port,
                conversation_id
            )
        } else {
            format!("https://127.0.0.1:{}/chat/v6/messages", port)
        };

        match self
            .local_client
            .get(&url)
            .header("Authorization", &auth)
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status();
                if !status.is_success() {
                    tracing::debug!(
                        "[get_chat_history] HTTP {}: {}",
                        status,
                        resp.text().await.unwrap_or_default()
                    );
                    return None;
                }

                let raw = resp.text().await.unwrap_or_default();
                let history = parse_chat_history_json(&raw);
                if let Some(ref history) = history {
                    tracing::debug!(
                        "[get_chat_history] {} message(s) cid={:?}",
                        history.messages.len(),
                        cid
                    );
                }
                history
            }
            Err(e) => {
                tracing::debug!("[get_chat_history] Request error: {}", e);
                None
            }
        }
    }


    /// Send a chat message
    pub async fn send_chat_message(
        &self,
        cid: &str,
        message: &str,
        message_type: &str, // "chat", "groupchat", or "system"
    ) -> Option<SendChatResponse> {
        let port = self.local_port.read().clone();
        let auth = self.local_auth.read().clone();
        let url = format!("https://127.0.0.1:{}/chat/v6/messages", port);

        let body = SendChatRequest {
            cid: cid.to_string(),
            message: message.to_string(),
            message_type: message_type.to_string(),
        };

        match self
            .local_client
            .post(&url)
            .header("Authorization", &auth)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() {
                    resp.json().await.ok()
                } else {
                    let body = resp.text().await.unwrap_or_default();
                    tracing::warn!(
                        "[send_chat] {} {} -> {} {}",
                        message_type,
                        cid,
                        status,
                        body.chars().take(240).collect::<String>()
                    );
                    None
                }
            }
            Err(e) => {
                tracing::warn!("[send_chat] request error {cid}: {e}");
                None
            }
        }
    }


    /// Get friends list
    pub async fn get_friends(&self) -> Option<FriendsResponse> {
        let port = self.local_port.read().clone();
        let auth = self.local_auth.read().clone();
        let url = format!("https://127.0.0.1:{}/chat/v4/friends", port);

        match self
            .local_client
            .get(&url)
            .header("Authorization", &auth)
            .send()
            .await
        {
            Ok(resp) => resp.json().await.ok(),
            Err(_) => None,
        }
    }

    /// Pending friend requests (incoming + outgoing).
    pub async fn get_friend_requests(&self) -> Option<FriendRequestsResponse> {
        let port = self.local_port.read().clone();
        let auth = self.local_auth.read().clone();
        let url = format!("https://127.0.0.1:{}/chat/v4/friendrequests", port);

        match self
            .local_client
            .get(&url)
            .header("Authorization", &auth)
            .send()
            .await
        {
            Ok(resp) => {
                if resp.status().is_success() {
                    resp.json().await.ok()
                } else {
                    tracing::debug!(
                        "[get_friend_requests] HTTP {}",
                        resp.status()
                    );
                    None
                }
            }
            Err(e) => {
                tracing::debug!("[get_friend_requests] request error: {}", e);
                None
            }
        }
    }

    /// Send a friend request by Riot ID (name#tag).
    pub async fn send_friend_request(&self, game_name: &str, game_tag: &str) -> bool {
        let port = self.local_port.read().clone();
        let auth = self.local_auth.read().clone();
        let url = format!("https://127.0.0.1:{}/chat/v4/friendrequests", port);
        let body = SendFriendRequestBody {
            game_name: game_name.to_string(),
            game_tag: game_tag.to_string(),
        };

        match self
            .local_client
            .post(&url)
            .header("Authorization", &auth)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
        {
            Ok(resp) => {
                let ok = resp.status().is_success();
                if !ok {
                    tracing::warn!(
                        "[send_friend_request] HTTP {} for {}#{}",
                        resp.status(),
                        game_name,
                        game_tag
                    );
                }
                ok
            }
            Err(e) => {
                tracing::error!("[send_friend_request] request error: {}", e);
                false
            }
        }
    }

    /// Cancel an outgoing friend request by PUUID.
    pub async fn cancel_friend_request(&self, puuid: &str) -> bool {
        let port = self.local_port.read().clone();
        let auth = self.local_auth.read().clone();
        let url = format!("https://127.0.0.1:{}/chat/v4/friendrequests", port);
        let body = RemoveFriendRequestBody {
            puuid: puuid.to_string(),
        };

        match self
            .local_client
            .delete(&url)
            .header("Authorization", &auth)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
        {
            Ok(resp) => {
                let ok = resp.status().is_success();
                if !ok {
                    tracing::warn!(
                        "[cancel_friend_request] HTTP {} for {}",
                        resp.status(),
                        puuid
                    );
                }
                ok
            }
            Err(e) => {
                tracing::error!("[cancel_friend_request] request error: {}", e);
                false
            }
        }
    }

    /// Generate conversation ID for a friend
    #[allow(dead_code)]
    pub fn generate_dm_cid(&self, friend_puuid: &str) -> String {
        let my_puuid = self.puuid.read().clone();
        let region = self.region.read().clone();

        // Sort PUUIDs alphabetically
        let (first, second) = if my_puuid < friend_puuid.to_string() {
            (my_puuid, friend_puuid.to_string())
        } else {
            (friend_puuid.to_string(), my_puuid)
        };

        format!("{}@{}.pvp.net-{}@{}.pvp.net", first, region, second, region)
    }

    /// Get chat participants
    pub async fn get_chat_participants(
        &self,
        cid: Option<&str>,
    ) -> Option<ChatParticipantsResponse> {
        let port = self.local_port.read().clone();
        let auth = self.local_auth.read().clone();

        let url = if let Some(conversation_id) = cid {
            format!(
                "https://127.0.0.1:{}/chat/v5/participants?cid={}",
                port, conversation_id
            )
        } else {
            format!("https://127.0.0.1:{}/chat/v5/participants", port)
        };

        match self
            .local_client
            .get(&url)
            .header("Authorization", &auth)
            .send()
            .await
        {
            Ok(resp) => resp.json().await.ok(),
            Err(_) => None,
        }
    }

    /// Generate all possible CID formats (prioritizing double format for DMs)
    #[allow(dead_code)]
    fn generate_all_cid_formats(&self, friend_puuid: &str) -> Vec<String> {
        let region = self.region.read().clone();
        let shard = self.shard.read().clone();

        // Helper to generate double format (REQUIRED for DM sends)
        let make_double = |r: &str| {
            let my_puuid = self.puuid.read().clone();
            let (first, second) = if my_puuid < friend_puuid.to_string() {
                (my_puuid.clone(), friend_puuid.to_string())
            } else {
                (friend_puuid.to_string(), my_puuid.clone())
            };
            format!("{}@{}.pvp.net-{}@{}.pvp.net", first, r, second, r)
        };

        // Priority regions to try first
        let priority_regions = vec![
            region.clone(),
            shard.clone(),
            "eu".to_string(),
            "na".to_string(),
        ];

        // All potentially valid chat regions/shards
        let mut all_regions = vec![
            "eu1".to_string(),
            "eu2".to_string(),
            "eu3".to_string(),
            "na1".to_string(),
            "ap1".to_string(),
            "ap2".to_string(),
            "ap".to_string(),
            "kr1".to_string(),
            "kr".to_string(),
            "br1".to_string(),
            "br".to_string(),
            "la1".to_string(),
            "la2".to_string(),
            "latam".to_string(),
            "tr1".to_string(),
            "tr".to_string(),
            "ru1".to_string(),
            "ru".to_string(),
        ];

        // Combine and dedup
        let mut regions_to_try = priority_regions;
        regions_to_try.append(&mut all_regions);
        regions_to_try.sort();
        regions_to_try.dedup();

        let mut formats = Vec::new();

        // CRITICAL: Double Formats ONLY for sending DMs
        // Single format is only for reading history, NOT for sending
        for r in &regions_to_try {
            formats.push(make_double(r));
        }

        formats
    }

    /// Find actual conversation ID for a friend from existing messages
    pub async fn find_dm_cid(&self, friend_puuid: &str) -> Option<String> {
        // Check all messages and find by PUUID
        if let Some(history) = self.get_chat_history(None).await {
            for msg in history.messages {
                // If we find a message with/to the friend in a 'chat'
                if msg.puuid == friend_puuid && msg.message_type == "chat" {
                    tracing::debug!("[find_dm_cid] Found CID from message: {}", msg.cid);
                    return Some(msg.cid);
                }
            }
        }
        None
    }

    /// Find or recover DM CID (tries finding, then probing generated formats)
    /// ALWAYS returns double format CID required for sending messages
    #[allow(dead_code)]
    pub async fn find_or_recover_dm_cid(&self, friend_puuid: &str) -> String {
        tracing::debug!(
            "[find_or_recover_dm_cid] Searching for friend: {}",
            friend_puuid
        );

        // 1. Try to find existing from messages (will return double format)
        if let Some(cid) = self.find_dm_cid(friend_puuid).await {
            tracing::info!("[find_or_recover_dm_cid] Found existing CID: {}", cid);
            return cid;
        }

        // 2. Generate double format CID (required for sending)
        let formats = self.generate_all_cid_formats(friend_puuid);
        tracing::debug!(
            "[find_or_recover_dm_cid] Generated {} double-format CIDs to try",
            formats.len()
        );

        // 3. Try the most likely format first (current region)
        let primary_cid = &formats[0];
        tracing::debug!(
            "[find_or_recover_dm_cid] Using primary double-format CID: {}",
            primary_cid
        );

        // We don't need to probe - just use the double format
        // The API will create the conversation on first send
        primary_cid.clone()
    }

    /// Initialize a DM conversation by sending an empty/test message to establish message_history
    /// This ensures the conversation is ready to receive messages
    #[allow(dead_code)]
    pub async fn initialize_dm_conversation(&self, cid: &str) -> bool {
        tracing::debug!(
            "[initialize_dm_conversation] Initializing conversation: {}",
            cid
        );

        // Try to get existing history first
        if let Some(history) = self.get_chat_history(Some(cid)).await {
            if !history.messages.is_empty() {
                tracing::info!("[initialize_dm_conversation] Conversation already has messages");
                return true;
            }
        }

        // Check if conversation exists and has message_history enabled
        if let Some(convs) = self.get_conversations().await {
            if let Some(conv) = convs.conversations.iter().find(|c| c.cid == cid) {
                if conv.message_history {
                    tracing::info!("[initialize_dm_conversation] Conversation exists with message_history=true");
                    return true;
                }
            }
        }

        tracing::debug!("[initialize_dm_conversation] ⚠ Conversation needs initialization, will send first message");
        // The conversation will be initialized when the first real message is sent
        true
    }

    /// Send DM to a friend by PUUID (Safely found CID with retry logic)
    #[allow(dead_code)]
    pub async fn send_dm(&self, friend_puuid: &str, message: &str) -> bool {
        tracing::debug!("[send_dm] Sending message to friend: {}", friend_puuid);

        // Use the recovery logic to find the best CID
        let cid = self.find_or_recover_dm_cid(friend_puuid).await;
        tracing::debug!("[send_dm] Using CID: {}", cid);

        // Initialize conversation if needed
        self.initialize_dm_conversation(&cid).await;

        // Try sending using the 'best' CID
        if self
            .send_chat_message(&cid, message, "chat")
            .await
            .is_some()
        {
            tracing::info!("[send_dm] Message sent successfully");
            return true;
        }

        tracing::debug!("[send_dm] ⚠ First attempt failed, trying all formats...");

        // Fallback: If that failed, try ALL formats blindly
        let formats = self.generate_all_cid_formats(friend_puuid);
        for (i, fmt_cid) in formats.iter().enumerate() {
            if fmt_cid == &cid {
                continue;
            } // Skip already tried

            tracing::debug!("[send_dm] Retry {}/{}: {}", i + 1, formats.len(), fmt_cid);

            if self
                .send_chat_message(fmt_cid, message, "chat")
                .await
                .is_some()
            {
                tracing::info!("[send_dm] Message sent with alternate format");
                return true;
            }
        }

        tracing::error!("[send_dm] All attempts failed");
        false
    }

    /// Get helper for single friend by PUUID
    #[allow(dead_code)]
    pub async fn get_friend_by_puuid(&self, puuid: &str) -> Option<Friend> {
        if let Some(friends) = self.get_friends().await {
            return friends.friends.into_iter().find(|f| f.puuid == puuid);
        }
        None
    }

    /// Check if conversation has message history
    #[allow(dead_code)]
    pub fn has_message_history(&self, conversation: &Conversation) -> bool {
        conversation.message_history
    }

    /// Get conversations with message history only
    #[allow(dead_code)]
    pub async fn get_conversations_with_history(&self) -> Vec<Conversation> {
        if let Some(convs) = self.get_conversations().await {
            return convs
                .conversations
                .into_iter()
                .filter(|c| c.message_history)
                .collect();
        }
        vec![]
    }

    /// Enhanced DM list - only shows conversations with history
    #[allow(dead_code)]
    pub async fn get_dm_list_with_history(&self) -> Vec<(String, Friend, i32, bool)> {
        let mut dm_list = Vec::new();

        if let Some(convs) = self.get_conversations().await {
            for conv in convs.conversations {
                // Only DMs
                if conv.conversation_type != "chat" || !conv.direct_messages {
                    continue;
                }

                // Get participants
                if let Some(participants) = self.get_chat_participants(Some(&conv.cid)).await {
                    for p in participants.participants {
                        if p.puuid == *self.puuid.read() {
                            continue;
                        }

                        if let Some(friend) = self.get_friend_by_puuid(&p.puuid).await {
                            dm_list.push((
                                conv.cid.clone(),
                                friend,
                                conv.unread_count,
                                conv.message_history, // History flag
                            ));
                        }
                    }
                }
            }
        }

        dm_list
    }

    /// Get DM history with checking for message_history flag
    #[allow(dead_code)]
    pub async fn get_dm_history_checked(
        &self,
        friend_puuid: &str,
    ) -> Result<ChatHistoryResponse, String> {
        // Find conversation
        let cid = self
            .find_dm_cid(friend_puuid)
            .await
            .ok_or("Conversation bulunamadı")?;

        // Check if conversation has history
        if let Some(convs) = self.get_conversations().await {
            let conv = convs
                .conversations
                .iter()
                .find(|c| c.cid == cid)
                .ok_or("Conversation detayları alınamadı")?;

            if !conv.message_history {
                return Err(
                    "Bu conversation için mesaj geçmişi devre dışı (message_history=false)"
                        .to_string(),
                );
            }
        }

        // Get history
        self.get_chat_history(Some(&cid))
            .await
            .ok_or("Mesaj geçmişi alınamadı".to_string())
    }

    /// Debug: Show all conversations with history status
    #[allow(dead_code)]
    pub async fn debug_message_history(&self) {
        tracing::debug!("=== MESSAGE HISTORY STATUS ===");

        if let Some(convs) = self.get_conversations().await {
            for conv in convs.conversations {
                tracing::debug!(
                    "CID: {} | Type: {} | DM: {} | History: {} | Unread: {}",
                    conv.cid,
                    conv.conversation_type,
                    conv.direct_messages,
                    if conv.message_history { "✓" } else { "✗" },
                    conv.unread_count
                );
            }
        }
    }

}


// Defense in depth: even a manual API command cannot reach Riot in diagnostic mode.
// An explicit local proxy overrides environment proxies and fails closed before TLS.
fn diagnostic_http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .proxy(reqwest::Proxy::all("http://127.0.0.1:0").expect("diagnostic sink"))
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(1))
        .build().expect("diagnostic HTTP client")
}

// Keep the insecure self-signed exception isolated to the local Riot API.
// Remote calls use the strict client and never inherit this exception.
fn remote_http_client() -> reqwest::Client {
    if crate::riot_offline_diagnostic() { return diagnostic_http_client(); }
    reqwest::Client::builder()
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(10))
        .connect_timeout(std::time::Duration::from_secs(5))
        .build()
        .expect("remote HTTP client")
}

fn local_riot_http_client() -> reqwest::Client {
    if crate::riot_offline_diagnostic() { return diagnostic_http_client(); }
    reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .https_only(true)
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(10))
        .connect_timeout(std::time::Duration::from_secs(5))
        .build()
        .expect("loopback Riot HTTP client")
}

#[cfg(test)]
mod personal_tls_tests {
    use super::{local_riot_http_client, remote_http_client};
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::time::{Duration, Instant};

    // Only a local TLS fixture; never contacts Riot or uses account credentials.
    fn tls_server(response: &'static [u8]) -> (String, std::thread::JoinHandle<()>) {
        let identity = native_tls::Identity::from_pkcs12(
            include_bytes!("../../tests/fixtures/localhost-test-only.p12"),
            "test-only",
        ).unwrap();
        let mut acceptor = native_tls::TlsAcceptor::builder(identity);
        acceptor.max_protocol_version(Some(native_tls::Protocol::Tlsv12));
        let acceptor = acceptor.build().unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("https://{}", listener.local_addr().unwrap());
        let worker = std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(10);
            while Instant::now() < deadline {
                match listener.accept() {
                    Ok((socket, _)) => {
                        // Winsock accepts inherit the listener's nonblocking mode.
                        socket.set_nonblocking(false).unwrap();
                        socket.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
                        socket.set_write_timeout(Some(Duration::from_secs(3))).unwrap();
                        // A strict client is expected to abort this untrusted handshake.
                        match acceptor.accept(socket) {
                            Ok(mut stream) => {
                                let mut request = [0_u8; 2048];
                                if stream.read(&mut request).is_ok() {
                                    let _ = stream.write_all(response);
                                }
                            }
                            Err(e) => eprintln!("test TLS handshake: {e:?}"),
                        }
                        return;
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Err(e) => panic!("test listener: {e}"),
                }
            }
            panic!("test client did not connect");
        });
        (url, worker)
    }

    #[tokio::test]
    async fn remote_rejects_self_signed_certificate() {
        let (url, worker) = tls_server(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nOK");
        let result = remote_http_client().get(&url).send().await;
        worker.join().unwrap();
        assert!(result.is_err(), "remote requests must validate the server certificate");
    }

    #[tokio::test]
    async fn local_riot_accepts_self_signed_certificate() {
        let (url, worker) = tls_server(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nOK");
        let response = local_riot_http_client().get(&url).send().await.unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        assert_eq!(response.text().await.unwrap(), "OK");
        worker.join().unwrap();
    }

    #[tokio::test]
    async fn local_riot_does_not_follow_redirects() {
        let (url, worker) = tls_server(b"HTTP/1.1 302 Found\r\nLocation: https://127.0.0.1:1/forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        let response = local_riot_http_client().get(&url).send().await.unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::FOUND);
        worker.join().unwrap();
    }

    #[tokio::test]
    async fn remote_rejects_plain_http() {
        let result = remote_http_client().get("http://127.0.0.1:1/").send().await;
        assert!(result.unwrap_err().is_builder());
    }
}

#[cfg(test)]
mod diagnostic_tests {
    #[tokio::test]
    async fn diagnostic_transport_cannot_reach_destination() {
        let target=std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        target.set_nonblocking(true).unwrap();
        let url=format!("http://{}/",target.local_addr().unwrap());
        assert!(super::diagnostic_http_client().get(url).send().await.is_err());
        assert_eq!(target.accept().unwrap_err().kind(),std::io::ErrorKind::WouldBlock);
    }
}

#[cfg(test)]
mod session_http_tests {
    use super::*;
    use std::{io::{Read,Write},net::TcpListener};
    fn fixture(statuses:Vec<u16>)->(String,std::thread::JoinHandle<()>) {
        let listener=TcpListener::bind("127.0.0.1:0").unwrap();
        let url=format!("http://{}/test",listener.local_addr().unwrap());
        let thread=std::thread::spawn(move || {
            for status in statuses {
                let (mut socket,_)=listener.accept().unwrap();
                socket.set_read_timeout(Some(std::time::Duration::from_secs(5))).unwrap();
                let mut buf=[0;8192];let _=socket.read(&mut buf).unwrap();
                let response=format!("HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{{}}");
                socket.write_all(response.as_bytes()).unwrap();
            }
        });
        (url,thread)
    }
    #[tokio::test]
    async fn repeated_permission_and_server_failures_preserve_session() {
        let statuses=vec![403,403,403,429,500,502,400,401,200];
        let (url,server)=fixture(statuses.clone());
        let mut api=ValorantAPI::new();api.client=reqwest::Client::builder().no_proxy().build().unwrap();
        for status in statuses {
            *api.needs_reinit.write()=false;
            let result=api.get_remote_ex::<serde_json::Value>(&url).await;
            assert_eq!(*api.needs_reinit.read(),status==401,"HTTP {status}");
            if status==429 {assert!(api.retry_after_secs().is_some_and(|seconds|seconds>0));}
            if status==200 {assert!(matches!(result,RemoteResult::Ok(_)));}
            else {assert!(matches!(result,RemoteResult::Transient));}
        }
        server.join().unwrap();
    }
    #[tokio::test]
    async fn autolock_transport_stays_enabled_and_does_not_refresh_on_refusal() {
        let statuses=vec![403,429,401,200];let (url,server)=fixture(statuses.clone());
        let mut api=ValorantAPI::new();api.client=reqwest::Client::builder().no_proxy().build().unwrap();
        for status in statuses {
            *api.needs_reinit.write()=false;
            let result=api.post_remote_silent(&url).await;
            assert_eq!(*api.needs_reinit.read(),status==401);
            assert_eq!(result.is_some(),status==200);
        }
        server.join().unwrap();
    }
}

#[cfg(test)]
mod match_detail_gate_tests {
    use super::*;
    #[tokio::test]
    async fn cache_is_free_and_queued_duplicates_recheck_before_network() {
        let api=std::sync::Arc::new(ValorantAPI::new());
        let gate=api.match_details_gate.lock().await;
        let other=api.clone();
        let queued=tokio::spawn(async move {other.get_match_details("shared").await});
        tokio::task::yield_now().await;
        let fixture=MatchDetailsResponse {round_results:None,match_info:None,players:Some(vec![]),teams:None};
        api.match_details_cache.write().insert("shared".into(),fixture);
        // Cached data remains readable even during a real rate-limit window.
        *api.rate_limit_until.write()=Some(std::time::Instant::now()+std::time::Duration::from_secs(60));
        drop(gate);
        assert!(queued.await.unwrap().is_some());
        let gate=api.match_details_gate.lock().await;
        assert!(tokio::time::timeout(std::time::Duration::from_millis(100),api.get_match_details("shared")).await.unwrap().is_some());
        drop(gate);
    }
    #[tokio::test]
    async fn stats_metadata_uses_one_budget_and_stops_on_rate_limit() {
        let api=std::sync::Arc::new(ValorantAPI::new());
        let started=std::time::Instant::now();
        let reads=(0..3).map(|_|{let api=api.clone();async move {assert!(api.wait_for_stats_read().await);started.elapsed()}});
        let mut times=futures_util::future::join_all(reads).await;times.sort();
        assert!(times[1].saturating_sub(times[0])>=std::time::Duration::from_millis(450));
        assert!(times[2].saturating_sub(times[1])>=std::time::Duration::from_millis(450));
        *api.rate_limit_until.write()=Some(std::time::Instant::now()+std::time::Duration::from_secs(60));
        assert!(!api.wait_for_stats_read().await);
    }

}
