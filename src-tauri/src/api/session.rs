//! Scheduling hints only; JWT claims never grant permissions or bypass MFA.
use base64::{engine::general_purpose::{URL_SAFE, URL_SAFE_NO_PAD}, Engine};
use std::time::Duration;

pub fn refresh_after(access: &str, entitlement: &str, now: u64) -> Duration {
    let expiry = [access, entitlement].into_iter().filter_map(|token| {
        let payload=token.split('.').nth(1)?;
        let data=URL_SAFE_NO_PAD.decode(payload).or_else(|_|URL_SAFE.decode(payload)).ok()?;
        let claims:serde_json::Value=serde_json::from_slice(&data).ok()?;
        claims.get("exp")?.as_u64()
    }).min();
    Duration::from_secs(expiry.map(|exp| exp.saturating_sub(now).saturating_sub(30)).unwrap_or(45*60))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn jwt(exp:u64)->String {format!("e30.{}.sig",URL_SAFE_NO_PAD.encode(format!("{{\"exp\":{exp}}}")))}
    #[test] fn uses_real_expiry_instead_of_arbitrary_45_minutes() {
        assert_eq!(refresh_after(&jwt(8200),&jwt(11800),1000),Duration::from_secs(7170));
        assert_eq!(refresh_after(&jwt(8200),&jwt(1600),1000),Duration::from_secs(570));
    }
    #[test] fn expired_and_nearly_expired_tokens_do_not_underflow() {
        assert_eq!(refresh_after(&jwt(999),"opaque",1000),Duration::ZERO);
        assert_eq!(refresh_after(&jwt(1010),"opaque",1000),Duration::ZERO);
    }
    #[test] fn malformed_or_opaque_tokens_use_bounded_fallback() {
        assert_eq!(refresh_after("invalid","x.!.y",1000),Duration::from_secs(2700));
        assert_eq!(refresh_after("invalid",&jwt(1600),1000),Duration::from_secs(570));
    }
}
