use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Profile {
    pub puuid: String,
    pub name: Option<String>,
    pub region: String,
    pub level: Option<u32>,
}
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Rank {
    pub tier: Option<u32>,
    pub rr: Option<u32>,
    pub peak: Option<u32>,
}
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct PlayerStats {
    pub puuid: String,
    pub name: Option<String>,
    pub agent: String,
    pub team: String,
    pub tier: Option<u32>,
    pub is_me: bool,
    pub kills: Option<u32>,
    pub deaths: Option<u32>,
    pub assists: Option<u32>,
    pub score: Option<u32>,
    pub rounds: Option<u32>,
    pub headshots: Option<u32>,
    pub bodyshots: Option<u32>,
    pub legshots: Option<u32>,
    pub damage: Option<u32>,
}
impl PlayerStats {
    pub fn kd(&self) -> Option<f64> {
        ratio(self.kills?, self.deaths?)
    }
    pub fn acs(&self) -> Option<f64> {
        ratio(self.score?, self.rounds?)
    }
    pub fn hs(&self) -> Option<f64> {
        Some(
            ratio(
                self.headshots?,
                self.headshots?
                    .checked_add(self.bodyshots?)?
                    .checked_add(self.legshots?)?,
            )? * 100.0,
        )
    }
    pub fn adr(&self) -> Option<f64> {
        ratio(self.damage?, self.rounds?)
    }
}
pub fn ratio(n: u32, d: u32) -> Option<f64> {
    (d > 0).then(|| n as f64 / d as f64)
}
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Match {
    pub id: String,
    pub map: String,
    pub queue: String,
    pub started_at: Option<u64>,
    pub duration_ms: Option<u64>,
    pub won: Option<bool>,
    pub ally_score: Option<u32>,
    pub enemy_score: Option<u32>,
    pub me: PlayerStats,
    pub players: Vec<PlayerStats>,
}
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Snapshot {
    pub schema: u32,
    pub provider: String,
    pub profile: Option<Profile>,
    pub rank: Option<Rank>,
    pub matches: Vec<Match>,
    pub updated_at: Option<u64>,
    pub partial: bool,
    pub status: String,
    pub retry_at: Option<u64>,
}
pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
pub fn parse_session(details: crate::api::types::MatchDetailsResponse, who: &str) -> Option<Match> {
    let info = details.match_info?;
    if info.is_completed == Some(false) {
        return None;
    }
    let id = info.match_id.filter(|s| !s.is_empty())?;
    let teams = details.teams.unwrap_or_default();
    let raw = details.players?;
    let players: Vec<_> = raw
        .into_iter()
        .filter(|p| p.is_observer != Some(true))
        .map(|p| {
            let stats = p.stats;
            let shots = p.round_damage;
            let sum = |f: fn(&crate::api::types::RoundDamage) -> Option<i32>| {
                shots.as_ref().and_then(|rows| {
                    if rows.is_empty() {
                        return None;
                    }
                    rows.iter().try_fold(0u32, |n, r| {
                        Some(n.saturating_add(u32::try_from(f(r)?).ok()?))
                    })
                })
            };
            let stat = |f: fn(&crate::api::types::MatchPlayerStats) -> Option<i32>| {
                stats
                    .as_ref()
                    .and_then(f)
                    .and_then(|n| u32::try_from(n).ok())
            };
            let name = p
                .game_name
                .zip(p.tag_line)
                .filter(|(n, t)| !n.is_empty() && !t.is_empty())
                .map(|(n, t)| format!("{n}#{t}"));
            PlayerStats {
                is_me: p.subject == who,
                puuid: p.subject,
                name,
                agent: p
                    .character_id
                    .as_deref()
                    .map(crate::agent_catalog::name)
                    .unwrap_or_default(),
                team: p.team_id.unwrap_or_default(),
                tier: p.competitive_tier.and_then(|n| u32::try_from(n).ok()),
                kills: stat(|s| s.kills),
                deaths: stat(|s| s.deaths),
                assists: stat(|s| s.assists),
                score: stat(|s| s.score),
                rounds: stat(|s| s.rounds_played),
                headshots: sum(|s| s.headshots),
                bodyshots: sum(|s| s.bodyshots),
                legshots: sum(|s| s.legshots),
                damage: sum(|s| s.damage),
            }
        })
        .collect();
    let me = players.iter().find(|p| p.is_me)?.clone();
    let ally = teams
        .iter()
        .find(|t| t.team_id.as_deref() == Some(&me.team));
    let enemy = teams
        .iter()
        .find(|t| t.team_id.as_deref() != Some(&me.team));
    let own = ally
        .and_then(|t| t.num_points)
        .and_then(|v| u32::try_from(v).ok());
    let other = enemy
        .and_then(|t| t.num_points)
        .and_then(|v| u32::try_from(v).ok());
    let won = if own.zip(other).is_some_and(|(a, b)| a == b) {
        None
    } else {
        ally.and_then(|t| t.won)
            .or_else(|| own.zip(other).map(|(a, b)| a > b))
    };
    let map = info.map_id.unwrap_or_default();
    let map = crate::constants::MAP_NAMES
        .get(map.as_str())
        .map(|v| v.to_string())
        .unwrap_or_else(|| map.rsplit('/').next().unwrap_or("").to_owned());
    let mut value = Match {
        id,
        map,
        queue: info.queue_id.unwrap_or_default(),
        started_at: info.game_start_millis,
        duration_ms: info.game_length_millis,
        won,
        ally_score: own,
        enemy_score: other,
        me,
        players,
    };
    for p in &mut value.players {
        if !p.is_me {
            p.kills = None;
            p.deaths = None;
            p.assists = None;
            p.score = None;
            p.rounds = None;
            p.headshots = None;
            p.bodyshots = None;
            p.legshots = None;
            p.damage = None;
        }
    }
    Some(value)
}

pub fn sanitize_snapshot(snapshot: &mut Snapshot) {
    for game in &mut snapshot.matches {
        for player in &mut game.players {
            if player.puuid != game.me.puuid {
                player.kills = None;
                player.deaths = None;
                player.assists = None;
                player.score = None;
                player.rounds = None;
                player.headshots = None;
                player.bodyshots = None;
                player.legshots = None;
                player.damage = None;
            }
        }
    }
}
