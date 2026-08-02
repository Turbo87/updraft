use super::*;
use std::sync::atomic::AtomicUsize;

/// A [`WindowRebuild`] whose answers a test schedules against the number of
/// offers made so far.
///
/// `build_failed` is not schedulable: it is set by [`Fake::build`], the way
/// [`ConfiguredWindows::build`] sets it, so a test cannot ask for a failure
/// that never built.
#[derive(Clone, Default)]
struct Fake {
    offers: Arc<AtomicUsize>,
    builds: Arc<AtomicUsize>,
    failed: Arc<AtomicBool>,
    window_after: Option<usize>,
    activity_gone_after: Option<usize>,
    build_fails: bool,
    unreachable: bool,
}

impl Fake {
    fn offers(&self) -> usize {
        self.offers.load(Ordering::SeqCst)
    }

    fn builds(&self) -> usize {
        self.builds.load(Ordering::SeqCst)
    }

    fn reached(&self, scheduled: Option<usize>) -> bool {
        scheduled.is_some_and(|offers| self.offers() >= offers)
    }
}

impl WindowRebuild for Fake {
    type Error = &'static str;

    fn window_exists(&self) -> bool {
        self.reached(self.window_after)
    }

    fn activity_exists(&self) -> bool {
        !self.reached(self.activity_gone_after)
    }

    fn build_failed(&self) -> bool {
        self.failed.load(Ordering::SeqCst)
    }

    fn build(&self) {
        self.builds.fetch_add(1, Ordering::SeqCst);
        if self.build_fails {
            self.failed.store(true, Ordering::SeqCst);
        }
    }

    fn offer(&self) -> Result<(), &'static str> {
        if self.unreachable {
            return Err("the event loop is closed");
        }
        self.offers.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

// A paused clock runs the rebuild's whole patience without waiting it out.
#[tokio::test(start_paused = true)]
async fn window_that_already_exists_costs_no_offer() {
    let fake = Fake {
        window_after: Some(0),
        ..Fake::default()
    };

    assert_eq!(rebuild(fake.clone()).await, Outcome::Window);
    assert_eq!(fake.offers(), 0);
}

#[tokio::test(start_paused = true)]
async fn dropped_offer_is_repeated_until_the_window_appears() {
    let fake = Fake {
        window_after: Some(3),
        ..Fake::default()
    };

    assert_eq!(rebuild(fake.clone()).await, Outcome::Window);
    assert_eq!(fake.offers(), 3);
}

/// A window that appears only on the last offer still counts: the rebuild
/// re-checks once the loop is spent rather than reporting [`Outcome::GaveUp`].
#[tokio::test(start_paused = true)]
async fn window_that_only_the_last_offer_produces_still_counts() {
    let fake = Fake {
        window_after: Some(OFFERS),
        ..Fake::default()
    };

    assert_eq!(rebuild(fake.clone()).await, Outcome::Window);
    assert_eq!(fake.offers(), OFFERS);
}

#[tokio::test(start_paused = true)]
async fn window_that_never_appears_gives_up_after_the_last_offer() {
    let fake = Fake::default();

    assert_eq!(rebuild(fake.clone()).await, Outcome::GaveUp);
    assert_eq!(fake.offers(), OFFERS);
}

/// A destroyed activity has to stop the rebuild rather than let it keep
/// offering: an offer that lands inside the next activity's `onCreate`
/// aborts the process.
#[tokio::test(start_paused = true)]
async fn destroyed_activity_stops_the_rebuild() {
    let fake = Fake {
        activity_gone_after: Some(2),
        ..Fake::default()
    };

    assert_eq!(rebuild(fake.clone()).await, Outcome::ActivityGone);
    assert_eq!(fake.offers(), 2);
}

/// A build that failed fails the same way every time, so repeating it only
/// buries the reason under its own repetitions.
///
/// Driven through the event loop's own entry point, so the failure the loop
/// stops on is the one a build actually recorded.
#[tokio::test(start_paused = true)]
async fn failed_build_stops_the_rebuild() {
    let fake = Fake {
        build_fails: true,
        ..Fake::default()
    };

    take_offer(&fake);
    assert_eq!(fake.builds(), 1);

    assert_eq!(rebuild(fake.clone()).await, Outcome::BuildFailed);
    assert_eq!(fake.offers(), 0);
}

/// The offers still queued when a build fails reach the event loop after the
/// loop has given up on them, and must not repeat the failure.
#[test]
fn offer_queued_before_a_failed_build_builds_nothing() {
    let fake = Fake {
        build_fails: true,
        ..Fake::default()
    };

    take_offer(&fake);
    take_offer(&fake);

    assert_eq!(fake.builds(), 1);
}

#[tokio::test(start_paused = true)]
async fn unreachable_event_loop_stops_the_rebuild() {
    let fake = Fake {
        unreachable: true,
        ..Fake::default()
    };

    assert_eq!(rebuild(fake.clone()).await, Outcome::LoopClosed);
}

/// The guard that closes the abort. A rebuild offered for an activity that
/// has since been destroyed runs anyway, whenever the event loop next
/// wakes, and must build nothing when it does.
#[test]
fn offer_that_outlived_its_activity_builds_nothing() {
    let fake = Fake {
        activity_gone_after: Some(0),
        ..Fake::default()
    };

    take_offer(&fake);

    assert_eq!(fake.builds(), 0);
}

#[test]
fn offer_that_finds_a_window_builds_nothing() {
    let fake = Fake {
        window_after: Some(0),
        ..Fake::default()
    };

    take_offer(&fake);

    assert_eq!(fake.builds(), 0);
}

/// The control the two above need: without it they would both pass against
/// a [`take_offer`] that never builds at all.
#[test]
fn offer_with_an_attached_activity_and_no_window_builds() {
    let fake = Fake::default();

    take_offer(&fake);

    assert_eq!(fake.builds(), 1);
}
