use docker_watchdog::restarter::RestartTracker;

// ─── Regression test: infinite restart loop bug ───
// Previously, the watcher reset the restart counter on every "start"
// event, including restarts WE triggered. This caused the attempt to
// always stay at 1/N, leading to infinite restarts.

#[test]
fn test_counter_increments_on_consecutive_failures() {
    let mut tracker = RestartTracker::new(3, 0, false, 0, false);
    let id = "abc123";
    let name = "my-app";

    assert!(tracker.should_restart(id, name, None));  // attempt 1/3
    assert_eq!(tracker.restart_count(id), 1);

    assert!(tracker.should_restart(id, name, None));  // attempt 2/3
    assert_eq!(tracker.restart_count(id), 2);

    assert!(tracker.should_restart(id, name, None));  // attempt 3/3
    assert_eq!(tracker.restart_count(id), 3);

    // 4th attempt should be DENIED
    assert!(!tracker.should_restart(id, name, None));
    assert_eq!(tracker.restart_count(id), 3);
}

#[test]
fn test_counter_not_reset_between_attempts() {
    let mut tracker = RestartTracker::new(2, 0, false, 0, false);
    let id = "container_x";

    assert!(tracker.should_restart(id, "app", None));   // 1/2
    // tracker.reset(id);   // ← THIS WAS THE BUG — DO NOT DO THIS
    assert!(tracker.should_restart(id, "app", None));   // 2/2
    assert!(!tracker.should_restart(id, "app", None));  // denied!
}

#[test]
fn test_reset_allows_fresh_restarts() {
    let mut tracker = RestartTracker::new(2, 0, false, 0, false);
    let id = "container_y";

    assert!(tracker.should_restart(id, "app", None));   // 1/2
    assert!(tracker.should_restart(id, "app", None));   // 2/2
    assert!(!tracker.should_restart(id, "app", None));  // denied

    tracker.reset(id);
    assert_eq!(tracker.restart_count(id), 0);
    assert!(tracker.should_restart(id, "app", None));   // 1/2 again
}

#[test]
fn test_different_containers_tracked_independently() {
    let mut tracker = RestartTracker::new(1, 0, false, 0, false);

    assert!(tracker.should_restart("container_a", "a", None));
    assert!(!tracker.should_restart("container_a", "a", None));

    assert!(tracker.should_restart("container_b", "b", None));
    assert!(!tracker.should_restart("container_b", "b", None));
}

#[test]
fn test_restart_count_starts_at_zero() {
    let tracker = RestartTracker::new(5, 10, false, 0, false);
    assert_eq!(tracker.restart_count("nonexistent"), 0);
}

#[test]
fn test_max_restarts_zero_means_never_restart() {
    let mut tracker = RestartTracker::new(0, 0, false, 0, false);
    assert!(!tracker.should_restart("any", "any", None));
}

#[test]
fn test_cleanup_stale_removes_old_entries() {
    use std::time::Duration;
    let mut tracker = RestartTracker::new(3, 0, false, 0, false);

    assert!(tracker.should_restart("old_container", "old", None));
    assert_eq!(tracker.restart_count("old_container"), 1);

    tracker.cleanup_stale(Duration::from_secs(0));
    assert_eq!(tracker.restart_count("old_container"), 0);
}

// ─── Per-container label override tests ───

#[test]
fn test_max_override_allows_more_restarts() {
    let mut tracker = RestartTracker::new(1, 0, false, 0, false);

    // Global limit is 1, but override allows 3
    assert!(tracker.should_restart("special", "app", Some(3)));  // 1/3
    assert!(tracker.should_restart("special", "app", Some(3)));  // 2/3
    assert!(tracker.should_restart("special", "app", Some(3)));  // 3/3
    assert!(!tracker.should_restart("special", "app", Some(3))); // denied at 3
}

#[test]
fn test_max_override_limits_restarts() {
    let mut tracker = RestartTracker::new(10, 0, false, 0, false);

    // Global limit is 10, but override restricts to 1
    assert!(tracker.should_restart("restricted", "app", Some(1)));   // 1/1
    assert!(!tracker.should_restart("restricted", "app", Some(1)));  // denied at 1
}
