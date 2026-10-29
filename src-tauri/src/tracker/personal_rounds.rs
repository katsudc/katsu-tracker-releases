use crate::api::types::MatchDetailsResponse;
use std::collections::{HashMap,HashSet};
/// Counts only unambiguous first opposing-player eliminations. Missing round data stays unknown.
pub fn opening_duels(details:&MatchDetailsResponse,who:&str)->Option<(u32,u32,u32)> {
    let rounds=details.round_results.as_ref()?;
    if rounds.is_empty() || rounds.len()>100 {return None;}
    let teams:HashMap<_,_>=details.players.as_ref()?.iter().filter_map(|p|Some((p.subject.as_str(),p.team_id.as_deref()?))).collect();
    if !teams.contains_key(who){return None;}
    let expected=details.players.as_ref()?.iter().find(|p|p.subject==who)?.stats.as_ref()?.rounds_played?;
    if expected<=0 || rounds.len()!=expected as usize{return None;}
    let (mut first_kills,mut first_deaths)=(0,0);
    for round in rounds {
        let players=round.player_stats.as_ref()?;
        if players.len()!=teams.len() || players.iter().filter_map(|p|p.subject.as_deref()).collect::<HashSet<_>>().len()!=teams.len() || !players.iter().all(|p|p.subject.as_deref().is_some_and(|s|teams.contains_key(s))){return None;}
        let mut unique=HashSet::new();
        for player in players {
            for kill in player.kills.as_ref()? {
                let (time,killer,victim)=(kill.round_time?,kill.killer.as_deref()?,kill.victim.as_deref()?);
                if killer==victim || killer.is_empty() || victim.is_empty(){continue;}
                let (Some(a),Some(b))=(teams.get(killer),teams.get(victim)) else {continue;};
                if a!=b {unique.insert((time,killer,victim));}
            }
        }
        let Some(earliest)=unique.iter().map(|k|k.0).min() else {continue;};
        let candidates:Vec<_>=unique.iter().filter(|k|k.0==earliest).collect();
        if candidates.len()!=1{return None;}
        let kill=candidates[0];if kill.1==who{first_kills+=1;}if kill.2==who{first_deaths+=1;}
    }
    Some((first_kills,first_deaths,rounds.len() as u32))
}

#[cfg(test)]
mod tests {
    #[test]
    fn companion_smoke_opening_requires_complete_rounds() {
        let mut data:MatchDetailsResponse=serde_json::from_value(serde_json::json!({"players":[{"subject":"me","teamId":"Blue","stats":{"roundsPlayed":1}},{"subject":"enemy","teamId":"Red"}],"roundResults":[{"playerStats":[{"subject":"me","kills":[{"roundTime":1000,"killer":"me","victim":"enemy"}]},{"subject":"enemy","kills":[]}]}]})).unwrap();
        assert_eq!(super::opening_duels(&data,"me"),Some((1,0,1)));
        data.round_results.as_mut().unwrap()[0].player_stats.as_mut().unwrap().pop();
        assert_eq!(super::opening_duels(&data,"me"),None);
    }
    use crate::api::types::MatchDetailsResponse;
}
