//! Tests for the CircuitBreaker implementation.

use kkr_runtime::CircuitBreaker;
use std::time::Duration;

// ---------------------------------------------------------------------------
// Initial state
// ---------------------------------------------------------------------------

#[test]
fn test_circuit_breaker_starts_closed() {
    let cb = CircuitBreaker::new(3, 2, Duration::from_secs(5));
    assert_eq!(cb.state(), "closed");
    assert!(!cb.is_open());
}

#[test]
fn test_circuit_breaker_initial_counts_are_zero() {
    let cb = CircuitBreaker::new(5, 3, Duration::from_secs(10));
    assert_eq!(cb.failure_count(), 0);
    assert_eq!(cb.success_count(), 0);
}

// ---------------------------------------------------------------------------
// Closed state -- allow_request
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_closed_state_allows_requests() {
    let cb = CircuitBreaker::new(3, 2, Duration::from_secs(5));
    assert!(cb.allow_request().await);
}

// ---------------------------------------------------------------------------
// Recording failures -- does not trip until threshold
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_failures_below_threshold_stay_closed() {
    let cb = CircuitBreaker::new(3, 2, Duration::from_secs(5));

    cb.record_failure().await;
    assert_eq!(cb.state(), "closed");
    assert_eq!(cb.failure_count(), 1);

    cb.record_failure().await;
    assert_eq!(cb.state(), "closed");
    assert_eq!(cb.failure_count(), 2);
}

#[tokio::test]
async fn test_single_failure_does_not_trip() {
    let cb = CircuitBreaker::new(5, 1, Duration::from_secs(5));
    cb.record_failure().await;
    assert_eq!(cb.state(), "closed");
    assert!(!cb.is_open());
}

// ---------------------------------------------------------------------------
// Enough failures trip to open
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_failures_at_threshold_trips_to_open() {
    let cb = CircuitBreaker::new(3, 2, Duration::from_secs(5));

    cb.record_failure().await;
    cb.record_failure().await;
    cb.record_failure().await;

    assert_eq!(cb.state(), "open");
    assert!(cb.is_open());
    assert_eq!(cb.failure_count(), 3);
}

#[tokio::test]
async fn test_exactly_at_threshold_trips() {
    let cb = CircuitBreaker::new(2, 1, Duration::from_secs(5));

    cb.record_failure().await;
    cb.record_failure().await;

    assert_eq!(cb.state(), "open");
    assert!(cb.is_open());
}

#[tokio::test]
async fn test_threshold_of_one_trips_immediately() {
    let cb = CircuitBreaker::new(1, 1, Duration::from_secs(5));
    cb.record_failure().await;
    assert!(cb.is_open());
}

// ---------------------------------------------------------------------------
// Open state rejects requests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_open_state_rejects_requests() {
    let cb = CircuitBreaker::new(1, 1, Duration::from_secs(60));

    // Trip the breaker
    cb.record_failure().await;
    assert!(cb.is_open());

    // Should reject
    assert!(!cb.allow_request().await);
}

#[tokio::test]
async fn test_open_state_rejects_multiple_requests() {
    let cb = CircuitBreaker::new(1, 1, Duration::from_secs(60));

    cb.record_failure().await;

    for _ in 0..5 {
        assert!(!cb.allow_request().await);
    }

    // Should still be open
    assert!(cb.is_open());
}

// ---------------------------------------------------------------------------
// After reset_timeout, transitions to half-open
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_transitions_to_half_open_after_timeout() {
    // Use a very short timeout for testing
    let cb = CircuitBreaker::new(1, 1, Duration::from_millis(50));

    // Trip the breaker
    cb.record_failure().await;
    assert!(cb.is_open());

    // Wait for reset timeout to elapse
    tokio::time::sleep(Duration::from_millis(100)).await;

    // allow_request should transition to half_open and return true
    let allowed = cb.allow_request().await;
    assert!(allowed);
    assert_eq!(cb.state(), "half_open");
}

#[tokio::test]
async fn test_does_not_transition_before_timeout() {
    let cb = CircuitBreaker::new(1, 1, Duration::from_secs(60));

    cb.record_failure().await;
    assert!(cb.is_open());

    // Immediately check -- timeout has not elapsed
    assert!(!cb.allow_request().await);
    assert_eq!(cb.state(), "open");
}

// ---------------------------------------------------------------------------
// Half-open state: success transitions to closed
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_success_in_half_open_transitions_to_closed() {
    let cb = CircuitBreaker::new(1, 1, Duration::from_millis(50));

    // Trip and wait for half-open
    cb.record_failure().await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(cb.allow_request().await);
    assert_eq!(cb.state(), "half_open");

    // Record success -- should transition to closed
    cb.record_success();

    assert_eq!(cb.state(), "closed");
    assert!(!cb.is_open());
    assert_eq!(cb.failure_count(), 0);
    assert_eq!(cb.success_count(), 0);
}

#[tokio::test]
async fn test_multiple_successes_needed_in_half_open() {
    // Require 3 successes to transition back to closed
    let cb = CircuitBreaker::new(1, 3, Duration::from_millis(50));

    // Trip and wait for half-open
    cb.record_failure().await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(cb.allow_request().await);
    assert_eq!(cb.state(), "half_open");

    // First two successes keep it half-open
    cb.record_success();
    assert_eq!(cb.state(), "half_open");
    assert_eq!(cb.success_count(), 1);

    cb.record_success();
    assert_eq!(cb.state(), "half_open");
    assert_eq!(cb.success_count(), 2);

    // Third success transitions to closed
    cb.record_success();
    assert_eq!(cb.state(), "closed");
}

// ---------------------------------------------------------------------------
// Half-open state: failure goes back to open
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_failure_in_half_open_goes_back_to_open() {
    let cb = CircuitBreaker::new(1, 1, Duration::from_millis(50));

    // Trip and wait for half-open
    cb.record_failure().await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(cb.allow_request().await);
    assert_eq!(cb.state(), "half_open");

    // Record failure -- should go back to open
    cb.record_failure().await;

    assert_eq!(cb.state(), "open");
    assert!(cb.is_open());
}

#[tokio::test]
async fn test_failure_in_half_open_resets_success_count() {
    let cb = CircuitBreaker::new(1, 3, Duration::from_millis(50));

    // Trip and wait for half-open
    cb.record_failure().await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(cb.allow_request().await);
    assert_eq!(cb.state(), "half_open");

    // Some successes then a failure
    cb.record_success();
    cb.record_success();
    assert_eq!(cb.success_count(), 2);

    cb.record_failure().await;
    assert_eq!(cb.state(), "open");
    assert_eq!(cb.success_count(), 0); // Reset by failure
}

// ---------------------------------------------------------------------------
// reset() returns to closed
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_reset_from_open_returns_to_closed() {
    let cb = CircuitBreaker::new(1, 1, Duration::from_secs(60));

    cb.record_failure().await;
    assert!(cb.is_open());

    cb.reset();

    assert_eq!(cb.state(), "closed");
    assert!(!cb.is_open());
    assert_eq!(cb.failure_count(), 0);
    assert_eq!(cb.success_count(), 0);
}

#[tokio::test]
async fn test_reset_from_half_open_returns_to_closed() {
    let cb = CircuitBreaker::new(1, 1, Duration::from_millis(50));

    cb.record_failure().await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(cb.allow_request().await);
    assert_eq!(cb.state(), "half_open");

    cb.reset();

    assert_eq!(cb.state(), "closed");
    assert_eq!(cb.failure_count(), 0);
    assert_eq!(cb.success_count(), 0);
}

#[test]
fn test_reset_from_closed_stays_closed() {
    let cb = CircuitBreaker::new(3, 2, Duration::from_secs(5));
    assert_eq!(cb.state(), "closed");

    cb.reset();

    assert_eq!(cb.state(), "closed");
    assert_eq!(cb.failure_count(), 0);
}

// ---------------------------------------------------------------------------
// Getter methods
// ---------------------------------------------------------------------------

#[test]
fn test_state_getter_closed() {
    let cb = CircuitBreaker::new(3, 2, Duration::from_secs(5));
    assert_eq!(cb.state(), "closed");
}

#[tokio::test]
async fn test_state_getter_open() {
    let cb = CircuitBreaker::new(1, 1, Duration::from_secs(60));
    cb.record_failure().await;
    assert_eq!(cb.state(), "open");
}

#[tokio::test]
async fn test_state_getter_half_open() {
    let cb = CircuitBreaker::new(1, 1, Duration::from_millis(50));
    cb.record_failure().await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    cb.allow_request().await;
    assert_eq!(cb.state(), "half_open");
}

#[tokio::test]
async fn test_failure_count_tracks_failures() {
    let cb = CircuitBreaker::new(10, 1, Duration::from_secs(5));

    for i in 1..=5 {
        cb.record_failure().await;
        assert_eq!(cb.failure_count(), i);
    }
}

#[tokio::test]
async fn test_success_count_tracks_in_half_open() {
    let cb = CircuitBreaker::new(1, 5, Duration::from_millis(50));

    cb.record_failure().await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    cb.allow_request().await;

    for i in 1..=4 {
        cb.record_success();
        assert_eq!(cb.success_count(), i);
    }
}

#[tokio::test]
async fn test_is_open_reflects_state() {
    let cb = CircuitBreaker::new(2, 1, Duration::from_secs(60));

    // Closed
    assert!(!cb.is_open());

    // Still closed
    cb.record_failure().await;
    assert!(!cb.is_open());

    // Open
    cb.record_failure().await;
    assert!(cb.is_open());

    // Reset to closed
    cb.reset();
    assert!(!cb.is_open());
}

// ---------------------------------------------------------------------------
// Success in closed state resets failure count
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_success_in_closed_resets_failure_count() {
    let cb = CircuitBreaker::new(3, 1, Duration::from_secs(5));

    cb.record_failure().await;
    cb.record_failure().await;
    assert_eq!(cb.failure_count(), 2);

    // Success should reset failure count
    cb.record_success();
    assert_eq!(cb.failure_count(), 0);
    assert_eq!(cb.state(), "closed");
}

// ---------------------------------------------------------------------------
// Half-open allows test requests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_half_open_allows_requests() {
    let cb = CircuitBreaker::new(1, 1, Duration::from_millis(50));

    cb.record_failure().await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    // First allow_request transitions to half_open
    assert!(cb.allow_request().await);
    assert_eq!(cb.state(), "half_open");

    // Subsequent requests in half_open should also be allowed
    assert!(cb.allow_request().await);
}

// ---------------------------------------------------------------------------
// Full lifecycle test
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_full_lifecycle_closed_open_halfopen_closed() {
    let cb = CircuitBreaker::new(2, 1, Duration::from_millis(50));

    // Phase 1: Closed
    assert_eq!(cb.state(), "closed");
    assert!(cb.allow_request().await);

    // Phase 2: Accumulate failures -> Open
    cb.record_failure().await;
    cb.record_failure().await;
    assert_eq!(cb.state(), "open");
    assert!(!cb.allow_request().await);

    // Phase 3: Wait -> Half-Open
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(cb.allow_request().await);
    assert_eq!(cb.state(), "half_open");

    // Phase 4: Success -> Closed
    cb.record_success();
    assert_eq!(cb.state(), "closed");
    assert!(cb.allow_request().await);
}

#[tokio::test]
async fn test_full_lifecycle_with_relapse() {
    let cb = CircuitBreaker::new(2, 1, Duration::from_millis(50));

    // Trip the breaker
    cb.record_failure().await;
    cb.record_failure().await;
    assert_eq!(cb.state(), "open");

    // Wait and enter half-open
    tokio::time::sleep(Duration::from_millis(100)).await;
    cb.allow_request().await;
    assert_eq!(cb.state(), "half_open");

    // Fail again -> back to open
    cb.record_failure().await;
    assert_eq!(cb.state(), "open");

    // Wait again and enter half-open
    tokio::time::sleep(Duration::from_millis(100)).await;
    cb.allow_request().await;
    assert_eq!(cb.state(), "half_open");

    // Succeed this time -> closed
    cb.record_success();
    assert_eq!(cb.state(), "closed");
}
