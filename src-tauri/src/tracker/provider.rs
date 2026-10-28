use super::models::*;
use futures_util::future::BoxFuture;
use std::sync::Arc;
#[derive(Clone, Debug)]
pub struct ProviderError {
    pub code: &'static str,
    pub retry: u64,
}
impl ProviderError {
    pub fn new(code: &'static str) -> Self {
        Self { code, retry: 0 }
    }
}
pub type Progress = Arc<dyn Fn(Snapshot) + Send + Sync>;
pub trait StatsProvider: Send + Sync {
    fn fetch<'a>(
        &'a self,
        profile: Profile,
        seed: Snapshot,
        progress: Progress,
    ) -> BoxFuture<'a, Result<Snapshot, ProviderError>>;
}
pub struct SessionProvider(pub Arc<crate::api::ValorantAPI>);
impl StatsProvider for SessionProvider {
    fn fetch<'a>(
        &'a self,
        mut profile: Profile,
        mut seed: Snapshot,
        progress: Progress,
    ) -> BoxFuture<'a, Result<Snapshot, ProviderError>> {
        Box::pin(async move {
            let api = &self.0;
            if !*api.connected.read() {
                return Err(ProviderError::new("offline"));
            }
            let (rank, ids) = futures_util::future::join(
                api.get_rank_snapshot(&profile.puuid),
                api.get_competitive_match_ids(&profile.puuid, 20),
            )
            .await;

            if profile.name.is_none() && api.retry_after_secs().is_none() {
                profile.name = api
                    .get_player_names(&[profile.puuid.clone()])
                    .await
                    .remove(&profile.puuid);
            }
            seed.provider = "session".into();
            seed.profile = Some(profile.clone());
            seed.partial = true;
            seed.status = "refreshing".into();
            let rank_failed = rank.is_none();
            if let Some(r) = rank {
                seed.rank = Some(Rank {
                    tier: r.current_tier,
                    rr: if r.current_tier.is_some_and(|tier| tier > 0) {
                        r.current_rr
                    } else {
                        None
                    },
                    peak: (r.peak_tier > 0).then_some(r.peak_tier),
                });
            }
            progress(seed.clone());
            let ids = ids.ok_or_else(|| {
                api.retry_after_secs()
                    .map(|n| ProviderError {
                        code: "rate_limited",
                        retry: n as u64,
                    })
                    .unwrap_or_else(|| ProviderError::new("unavailable"))
            })?;
            let previous = seed.matches.clone();
            let mut matches = Vec::new();
            let mut failed = rank_failed;
            let mut seen = std::collections::HashSet::new();
            for id in &ids {
                if !seen.insert(id.clone()) {
                    continue;
                }
                if let Some(wait) = api.retry_after_secs() {
                    return Err(ProviderError {
                        code: "rate_limited",
                        retry: wait as u64,
                    });
                }
                let parsed = if let Some(d) = api.get_match_details(id).await {
                    parse_session(d, &profile.puuid)
                } else {
                    None
                };
                if let Some(m) = parsed {
                    matches.push(m);
                } else if let Some(old) = previous.iter().find(|m| &m.id == id) {
                    matches.push(old.clone());
                    failed = true;
                } else {
                    failed = true;
                }
                if matches.len() % 2 == 0 && !matches.is_empty() {
                    let mut partial = seed.clone();
                    partial.matches = matches.clone();
                    for old in &previous {
                        if partial.matches.len() < 20
                            && !partial.matches.iter().any(|m| m.id == old.id)
                        {
                            partial.matches.push(old.clone());
                        }
                    }
                    progress(partial);
                }
            }
            if !ids.is_empty() && matches.is_empty() {
                return Err(ProviderError::new("unavailable"));
            }
            matches.sort_by_key(|m| std::cmp::Reverse(m.started_at.unwrap_or(0)));
            seed.matches = matches;
            if let Some(name) = seed.matches.first().and_then(|m| m.me.name.clone()) {
                seed.profile.as_mut().unwrap().name = Some(name);
            }
            seed.partial = failed;
            seed.updated_at = Some(now());
            seed.status = if failed { "partial" } else { "ready" }.into();
            seed.retry_at = None;
            Ok(seed)
        })
    }
}
