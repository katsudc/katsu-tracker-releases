//! Initial/incomplete metadata is not a new round.
pub type Snapshot = (String, Option<String>, Option<i32>, Option<i32>);

#[derive(Default)]
pub struct SessionTracker {
    in_game: bool,
    match_id: Option<String>,
    score: Option<(i32, i32)>,
}

impl SessionTracker {
    /// True only for a known match change, known score change or confirmed exit.
    pub fn changed(&mut self, snapshot: Option<&Snapshot>, phase: &str) -> bool {
        let phase = snapshot.map_or(phase, |g|g.0.as_str());
        if phase == "idle" || phase == "pregame" {
            let ended = self.in_game;
            *self = Self::default();
            return ended;
        }
        // Missing/transient state does not erase the last complete score.
        let Some(g) = snapshot.filter(|g|g.0=="ingame") else { return false; };
        self.in_game = true;
        let mut changed = false;
        if let Some(id) = &g.1 {
            if self.match_id.as_ref().is_some_and(|old|old!=id) {
                changed = true;
                self.score = None;
            }
            self.match_id = Some(id.clone());
        }
        if let (Some(ally), Some(enemy)) = (g.2, g.3) {
            let score = (ally, enemy);
            changed |= self.score.is_some_and(|old|old!=score);
            self.score = Some(score);
        }
        changed
    }
}

pub fn should_capture(enabled: bool, in_game: bool, game_window_visible: bool, recording: bool) -> bool {
    recording || enabled && (in_game || game_window_visible)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn game(id: Option<&str>, ally: Option<i32>, enemy: Option<i32>) -> Snapshot {
        ("ingame".into(),id.map(str::to_string),ally,enemy)
    }
    #[test]
    fn first_snapshot_and_late_score_are_not_round_changes() {
        let mut tracker = SessionTracker::default();
        assert!(!tracker.changed(None,"idle"));
        assert!(!tracker.changed(Some(&game(None,None,None)),"ingame"));
        assert!(!tracker.changed(Some(&game(Some("match"),None,None)),"ingame"));
        assert!(!tracker.changed(Some(&game(Some("match"),Some(3),None)),"ingame"));
        assert!(!tracker.changed(Some(&game(Some("match"),Some(3),Some(2))),"ingame"));
    }
    #[test]
    fn missing_scores_do_not_reset_but_real_score_change_still_does() {
        let mut tracker = SessionTracker::default();
        let current = game(Some("match"),Some(3),Some(2));
        assert!(!tracker.changed(Some(&current),"ingame"));
        assert!(!tracker.changed(None,"ingame"));
        assert!(!tracker.changed(Some(&game(Some("match"),None,None)),"ingame"));
        assert!(!tracker.changed(Some(&current),"ingame"));
        let next = game(Some("match"),Some(4),Some(2));
        assert!(tracker.changed(Some(&next),"ingame"));
        assert!(!tracker.changed(Some(&next),"ingame"));
    }
    #[test]
    fn match_change_and_confirmed_exit_reset_once() {
        let mut tracker = SessionTracker::default();
        assert!(!tracker.changed(Some(&game(Some("a"),Some(0),Some(0))),"ingame"));
        assert!(tracker.changed(Some(&game(Some("b"),Some(0),Some(0))),"ingame"));
        assert!(tracker.changed(None,"idle"));
        assert!(!tracker.changed(None,"idle"));
        assert!(!tracker.changed(Some(&game(Some("c"),Some(0),Some(0))),"ingame"));
    }
    #[test]
    fn visible_game_starts_capture_without_waiting_for_api_and_respects_off() {
        assert!(should_capture(true,false,true,false));
        assert!(should_capture(true,true,false,false));
        assert!(!should_capture(false,true,true,false));
        assert!(!should_capture(true,false,false,false));
        assert!(should_capture(false,false,false,true));
    }
}
