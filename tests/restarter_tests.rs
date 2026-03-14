use docker_watchdog::restarter::RestartTracker;

// ─── Regression test: infinite restart loop bug ───
// Previously, the watcher reset the restart counter on every "start"
// event, including restarts WE triggered. This caused the attempt to
// always stay at 1/N, leading to infinite restarts.

#[test]
fn test_counter_increments_on_consecutive_failures() {
    let mut tracker = RestartTracker::new(3, 0);
    let id = "abc123";
    let name = "my-app";

    // Simulate 3 consecutive crash → restart cycles
    assert!(tracker.should_restart(id, name));  // attempt 1/3 → allowed
    assert_eq!(tracker.restart_count(id), 1);

    assert!(tracker.should_restart(id, name));  // attempt 2/3 → allowed
    assert_eq!(tracker.restart_count(id), 2);

    assert!(tracker.should_restart(id, name));  // attempt 3/3 → allowed
    assert_eq!(tracker.restart_count(id), 3);

    // 4th attempt should be DENIED — limit reached
    assert!(!tracker.should_restart(id, name));
    assert_eq!(tracker.restart_count(id), 3);   // count stays at 3, not reset
}

#[test]
fn test_counter_not_reset_between_attempts() {
    // This is the core regression test for the infinite restart bug.
    // Even if we DON'T call reset(), the counter must persist.
    let mut tracker = RestartTracker::new(2, 0);
    let id = "container_x";

    assert!(tracker.should_restart(id, "app"));   // 1/2
    // Simulating what the buggy code did: resetting on "start" event
    // tracker.reset(id);   // ← THIS WAS THE BUG — DO NOT DO THIS
    assert!(tracker.should_restart(id, "app"));   // 2/2
    assert!(!tracker.should_restart(id, "app"));  // denied!
}

#[test]
fn test_reset_allows_fresh_restarts() {
    let mut tracker = RestartTracker::new(2, 0);
    let id = "container_y";

    assert!(tracker.should_restart(id, "app"));   // 1/2
    assert!(tracker.should_restart(id, "app"));   // 2/2
    assert!(!tracker.should_restart(id, "app"));  // denied

    // Explicit reset (e.g., after container runs healthy for a while)
    tracker.reset(id);
    assert_eq!(tracker.restart_count(id), 0);
    assert!(tracker.should_restart(id, "app"));   // 1/2 again — allowed
}

#[test]
fn test_different_containers_tracked_independently() {
    let mut tracker = RestartTracker::new(1, 0);

    assert!(tracker.should_restart("container_a", "a"));   // 1/1
    assert!(!tracker.should_restart("container_a", "a"));  // denied

    // container_b should be unaffected by container_a's limit
    assert!(tracker.should_restart("container_b", "b"));   // 1/1
    assert!(!tracker.should_restart("container_b", "b"));  // denied
}

#[test]
fn test_restart_count_starts_at_zero() {
    let tracker = RestartTracker::new(5, 10);
    assert_eq!(tracker.restart_count("nonexistent"), 0);
}

#[test]
fn test_max_restarts_zero_means_never_restart() {
    let mut tracker = RestartTracker::new(0, 0);
    assert!(!tracker.should_restart("any", "any"));
}
