use super::*;
use serde_json::json;

#[test]
fn session_stats_keep_missing_fields_and_join_teams_by_id() {
    let raw = json!({
        "matchInfo":{"matchId":"session-example","queueID":"competitive","isCompleted":true,"mapId":"/Game/Maps/Ascent/Ascent","gameStartMillis":1700000000000u64},
        "players":[
            {"subject":"me","teamId":"Blue","gameName":"Example","tagLine":"TEST","stats":{"kills":20,"deaths":10,"assists":5,"score":4400,"roundsPlayed":22},"roundDamage":[{"headshots":10,"bodyshots":25,"legshots":5,"damage":3000}]},
            {"subject":"other","teamId":"Red","stats":{"kills":99,"deaths":1,"assists":9,"score":9999,"roundsPlayed":22}}
        ],
        "teams":[{"teamId":"Red","numPoints":9,"won":false},{"teamId":"Blue","numPoints":13,"won":true}]
    });
    let parsed = parse_session(serde_json::from_value(raw.clone()).unwrap(), "me").unwrap();
    assert_eq!(parsed.won, Some(true));
    assert_eq!(parsed.ally_score, Some(13));
    assert_eq!(parsed.me.kd(), Some(2.0));
    assert_eq!(parsed.me.acs(), Some(200.0));
    assert_eq!(parsed.me.hs(), Some(25.0));
    assert!(parsed.me.adr().unwrap() > 136.0);
    assert_eq!(parsed.players[1].name, None);
    assert_eq!(parsed.players[1].kills, None);
    let mut draw = raw;
    draw["teams"][0]["numPoints"] = json!(13);
    assert_eq!(
        parse_session(serde_json::from_value(draw).unwrap(), "me")
            .unwrap()
            .won,
        None
    );
}

#[test]
fn cache_cleanup_removes_obsolete_credentials_without_touching_session_data() {
    let directory =
        std::env::temp_dir().join(format!("katsu-tracker-test-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&directory).unwrap();
    let path = directory.join("cache.json");
    let snapshot = Snapshot {
        schema: 1,
        provider: "session".into(),
        profile: Some(Profile {
            puuid: "example".into(),
            region: "eu".into(),
            ..Default::default()
        }),
        status: "cached".into(),
        ..Default::default()
    };
    save_json(&path, &snapshot).unwrap();
    save_json(&path, &snapshot).unwrap(); // Replace existing cache atomically on Windows.
    assert_eq!(
        read_json::<Snapshot>(&path).unwrap().profile.unwrap().puuid,
        "example"
    );
    std::fs::remove_file(&path).unwrap();
    let legacy_key = directory.join("henrik.key");
    let legacy_cache = directory.join("henrik-eu-example.json");
    std::fs::write(&legacy_key, "test-only-obsolete-value").unwrap();
    std::fs::write(&legacy_cache, "{}").unwrap();
    std::fs::write(directory.join("config.json"), "{}").unwrap();
    save_json(&path, &snapshot).unwrap();
    remove_legacy_provider_files(&directory);
    assert!(
        !legacy_key.exists() && !legacy_cache.exists() && !directory.join("config.json").exists()
    );
    assert!(session_cache(&path).is_some());
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir(directory).unwrap();
}
#[test]
fn rank_is_not_guessed_from_old_season_performance() {
    use crate::rank::current_rank;
    let old:crate::api::types::MmrResponse=serde_json::from_value(json!({"QueueSkills":{"competitive":{"SeasonalInfoBySeasonID":{"old":{"CompetitiveTier":27,"NumberOfGames":999,"RankedRating":500}}}}})).unwrap();
    assert_eq!(current_rank(&old), None);
    let unranked: crate::api::types::MmrResponse =
        serde_json::from_value(json!({"QueueSkills":{"competitive":{"CompetitiveTier":0}}}))
            .unwrap();
    assert_eq!(current_rank(&unranked), Some((0, 0)));
    let current:crate::api::types::MmrResponse=serde_json::from_value(json!({"LatestCompetitiveUpdate":{"TierAfterUpdate":23,"RankedRatingAfterUpdate":67},"QueueSkills":{"competitive":{"SeasonalInfoBySeasonID":{"old":{"CompetitiveTier":27,"NumberOfGames":999}}}}})).unwrap();
    assert_eq!(current_rank(&current), Some((23, 67)));
}
