#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;

#[test]
fn test_system_clock() {
    let clock = SystemClock::new();
    let now = clock.now_millis();
    assert!(now > 0);
}

#[test]
fn test_fake_clock_new() {
    let clock = FakeClock::new(1000);
    assert_eq!(clock.now_millis(), 1000);
}

#[test]
fn test_fake_clock_at_epoch() {
    let clock = FakeClock::at_epoch();
    assert_eq!(clock.now_millis(), 0);
}

#[test]
fn test_fake_clock_at_now() {
    let clock = FakeClock::at_now();
    let now = clock.now_millis();
    // Should be a reasonable recent timestamp
    assert!(now > 1700000000000); // After 2023
}

#[test]
fn test_fake_clock_advance() {
    let clock = FakeClock::new(1000);
    clock.advance(Duration::from_millis(500));
    assert_eq!(clock.now_millis(), 1500);
}

#[test]
fn test_fake_clock_advance_ms() {
    let clock = FakeClock::new(1000);
    clock.advance_ms(250);
    assert_eq!(clock.now_millis(), 1250);
}

#[test]
fn test_fake_clock_advance_secs() {
    let clock = FakeClock::new(1000);
    clock.advance_secs(5);
    assert_eq!(clock.now_millis(), 6000);
}

#[test]
fn test_fake_clock_set() {
    let clock = FakeClock::new(1000);
    clock.set(5000);
    assert_eq!(clock.now_millis(), 5000);
}

#[test]
fn test_fake_clock_set_duration() {
    let clock = FakeClock::new(0);
    clock.set_duration(Duration::from_secs(10));
    assert_eq!(clock.now_millis(), 10000);
}

#[test]
fn test_fake_clock_auto_advance_default() {
    let clock = FakeClock::new(0);
    assert!(clock.auto_advance());
}

#[test]
fn test_fake_clock_without_auto_advance() {
    let clock = FakeClock::new(0);
    let no_advance = clock.without_auto_advance();
    assert!(!no_advance.auto_advance());
    // Original unchanged
    assert!(clock.auto_advance());
}

#[tokio::test]
async fn test_fake_clock_sleep_auto_advance() {
    let clock = FakeClock::new(1000);
    clock.sleep(Duration::from_millis(500)).await;
    assert_eq!(clock.now_millis(), 1500);
}

#[tokio::test]
async fn test_fake_clock_sleep_no_auto_advance() {
    let mut clock = FakeClock::new(1000);
    clock.set_auto_advance(false);
    clock.sleep(Duration::from_millis(500)).await;
    assert_eq!(clock.now_millis(), 1000); // Unchanged
}

#[test]
fn test_fake_clock_shared_state() {
    let clock1 = FakeClock::new(1000);
    let clock2 = clock1.clone();

    clock1.advance_ms(500);
    assert_eq!(clock2.now_millis(), 1500);
}

#[test]
fn test_clock_handle_system() {
    let handle = ClockHandle::system();
    assert!(handle.is_system());
    assert!(!handle.is_fake());
    assert!(handle.as_fake().is_none());
}

#[test]
fn test_clock_handle_fake() {
    let handle = ClockHandle::fake_at(1000);
    assert!(handle.is_fake());
    assert!(!handle.is_system());

    let fake = handle.as_fake().unwrap();
    assert_eq!(fake.now_millis(), 1000);
}

#[test]
fn test_clock_handle_fake_at_epoch() {
    let handle = ClockHandle::fake_at_epoch();
    assert_eq!(handle.now_millis(), 0);
}

#[tokio::test]
async fn test_clock_handle_sleep() {
    let handle = ClockHandle::fake_at(1000);
    handle.sleep(Duration::from_millis(100)).await;
    assert_eq!(handle.now_millis(), 1100);
}

#[test]
fn test_clock_now_duration() {
    let clock = FakeClock::new(5000);
    let duration = clock.now();
    assert_eq!(duration, Duration::from_millis(5000));
}

#[test]
fn test_clock_handle_default() {
    let handle = ClockHandle::default();
    assert!(handle.is_system());
}
