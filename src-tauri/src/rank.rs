use crate::api::types::MmrResponse;
#[derive(Clone, Copy, Default)]
pub struct RankSnapshot {
    pub current_tier: Option<u32>,
    pub current_rr: Option<u32>,
    pub peak_tier: u32,
}
/// Never infer current rank from whichever historical act has the most games.
pub fn current_rank(data: &MmrResponse) -> Option<(u32, u32)> {
    if let Some(update) = data.latest_competitive_update.as_ref() {
        if let Some(tier) = update.tier_after_update {
            return Some((tier, update.ranked_rating_after_update.unwrap_or(0)));
        }
        if let Some(season) = update.season_id.as_ref().and_then(|id| {
            data.queue_skills
                .as_ref()?
                .competitive
                .as_ref()?
                .seasonal_info_by_season_id
                .as_ref()?
                .get(id)
        }) {
            if let Some(tier) = season.competitive_tier.or(season.rank) {
                return Some((tier, season.ranked_rating.unwrap_or(0)));
            }
        }
    }
    let competitive = data.queue_skills.as_ref()?.competitive.as_ref()?;
    competitive
        .competitive_tier
        .map(|tier| (tier, competitive.ranked_rating.unwrap_or(0)))
}
pub fn rank_snapshot_from_mmr(data: &MmrResponse) -> RankSnapshot {
    let current = current_rank(data);
    let mut result = RankSnapshot {
        current_tier: current.map(|r| r.0),
        current_rr: current.map(|r| r.1),
        peak_tier: 0,
    };
    if let Some(seasons) = data
        .queue_skills
        .as_ref()
        .and_then(|q| q.competitive.as_ref())
        .and_then(|c| c.seasonal_info_by_season_id.as_ref())
    {
        for (season_id, season) in seasons {
            let adjust = |tier: u32| {
                if tier > 20
                    && crate::constants::BEFORE_ASCENDANT_SEASONS.contains(&season_id.as_str())
                {
                    tier + 3
                } else {
                    tier
                }
            };
            result.peak_tier = result.peak_tier.max(adjust(season.effective_tier()));
            if let Some(wins) = season.wins_by_tier.as_ref() {
                for tier in wins.keys().filter_map(|s| s.parse::<u32>().ok()) {
                    result.peak_tier = result.peak_tier.max(adjust(tier));
                }
            }
        }
    }
    result
}
