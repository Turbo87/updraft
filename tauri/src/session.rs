use crate::driver::DriverHandle;
use tauri::ipc::{Channel, InvokeResponseBody};
use tauri_plugin_updraft::Fix as ReportedFix;
use tokio::sync::mpsc;
use updraft_core::{Fix as CoreFix, InternalGps};
use updraft_geo::LatLon;
use updraft_units::{Angle, EllipsoidAltitude, Length, Speed};

/// Builds the channel a platform session reports its GNSS fixes on.
///
/// The shell turns the platform's wire value into a domain value here, so the
/// core never learns what a platform fix looks like. Unlike a byte transport
/// there is no framing to defer to the core: the platform already delivers
/// structure.
pub fn fix_channel(handle: DriverHandle) -> Channel {
    let (sender, mut receiver) = mpsc::unbounded_channel();
    tauri::async_runtime::spawn(async move {
        while let Some(fix) = receiver.recv().await {
            let input = InternalGps::new(fix);
            if handle.send(input).await.is_err() {
                break;
            }
        }
    });

    Channel::new(move |body: InvokeResponseBody| {
        match body.deserialize::<ReportedFix>() {
            Ok(reported) => {
                let _ = sender.send(fix(reported));
            }
            // A fix that cannot be read is a map that stops moving, which
            // looks exactly like a receiver with no signal.
            Err(error) => tracing::error!(%error, "Discarded an unreadable GNSS fix"),
        }
        Ok(())
    })
}

fn fix(reported: ReportedFix) -> CoreFix {
    CoreFix {
        position: LatLon::from_degrees(reported.latitude_degrees, reported.longitude_degrees),
        altitude_ellipsoid: reported
            .altitude_ellipsoid_meters
            .map(Length::from_meters)
            .map(EllipsoidAltitude::new),
        track: reported.track_degrees.map(Angle::from_degrees),
        ground_speed: reported
            .ground_speed_meters_per_second
            .map(Speed::from_meters_per_second),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::driver::Driver;
    use approx::assert_abs_diff_eq;
    use claims::{assert_some, assert_some_eq};
    use std::time::Duration;
    use tokio::sync::mpsc;
    use tokio::time::{Instant, timeout, timeout_at};
    use tracing_test::traced_test;
    use updraft_core::{AirspaceState, Instruments, SettingsSnapshot, Topic};

    const PATIENCE: Duration = Duration::from_secs(5);

    const COMPLETE: &str = r#"{
        "latitudeDegrees": 50.823,
        "longitudeDegrees": 6.186,
        "altitudeEllipsoidMeters": 247.0,
        "trackDegrees": 270.0,
        "groundSpeedMetersPerSecond": 23.15
    }"#;

    /// `COMPLETE` with one optional field renamed, as a drifted
    /// `Location.toFix()` would send it.
    const RENAMED_OPTIONAL: &str = r#"{
        "latitudeDegrees": 50.823,
        "longitudeDegrees": 6.186,
        "altitudeEllipsoidMeters": 247.0,
        "trackDegreesss": 270.0,
        "groundSpeedMetersPerSecond": 23.15
    }"#;

    /// What the platform reports while the receiver has a position but no
    /// track, speed or altitude yet.
    const POSITION_ONLY: &str = r#"{
        "latitudeDegrees": 51.0,
        "longitudeDegrees": 7.0,
        "altitudeEllipsoidMeters": null,
        "trackDegrees": null,
        "groundSpeedMetersPerSecond": null
    }"#;

    fn driver() -> DriverHandle {
        Driver::spawn(
            SettingsSnapshot::default(),
            AirspaceState::none_at_startup(),
            Box::new(|_, _, _| Box::new(|| {})),
            Box::new(|_| {}),
            Duration::from_millis(100),
        )
    }

    fn topic_stream(handle: &DriverHandle) -> mpsc::UnboundedReceiver<Topic> {
        let (sender, receiver) = mpsc::unbounded_channel();
        handle.subscribe(Box::new(move |topic: &Topic| {
            sender.send(topic.clone()).is_ok()
        }));
        receiver
    }

    fn report(channel: &Channel, payload: &str) {
        channel
            .send(InvokeResponseBody::Json(payload.to_owned()))
            .expect("the channel accepts the payload");
    }

    async fn next_instruments(receiver: &mut mpsc::UnboundedReceiver<Topic>) -> Instruments {
        loop {
            let received = timeout(PATIENCE, receiver.recv())
                .await
                .expect("an instruments topic within the timeout");
            let Topic::Instruments(instruments) = assert_some!(received) else {
                continue;
            };
            return instruments;
        }
    }

    /// Awaits topics until one reports the given latitude, so neither the
    /// onboarding emission of empty state nor an earlier fix is counted.
    ///
    /// The deadline bounds the whole search rather than each receive, so a
    /// latitude that never arrives fails instead of spinning for as long as
    /// the core keeps emitting.
    async fn instruments_at(
        receiver: &mut mpsc::UnboundedReceiver<Topic>,
        latitude_degrees: f64,
    ) -> Instruments {
        let deadline = Instant::now() + PATIENCE;
        loop {
            let received = timeout_at(deadline, receiver.recv())
                .await
                .expect("a topic at that latitude within the timeout");
            let Topic::Instruments(instruments) = assert_some!(received) else {
                continue;
            };
            if instruments
                .position
                .is_some_and(|position| position.latitude_degrees == latitude_degrees)
            {
                return instruments;
            }
        }
    }

    #[tokio::test]
    async fn reported_fixes_reach_the_instruments_topic() {
        let handle = driver();
        let mut topics = topic_stream(&handle);

        report(&fix_channel(handle), COMPLETE);

        let instruments = instruments_at(&mut topics, 50.823).await;
        assert_eq!(
            assert_some!(instruments.position).longitude_degrees,
            6.186_f64
        );
        assert_some_eq!(instruments.track_degrees, 270.0_f64);
        assert_some_eq!(instruments.ground_speed_meters_per_second, 23.15_f64);

        // The geoid sits some 46.5 m above the ellipsoid here, so the MSL
        // altitude lands near 200 m. The tolerance leaves the core free to
        // refine its geoid model while still missing every other field of the
        // fix, the nearest of which is the uncorrected ellipsoidal altitude,
        // 46.5 m away.
        assert_abs_diff_eq!(
            assert_some!(instruments.altitude_msl_meters),
            200.5,
            epsilon = 10.0
        );
    }

    #[tokio::test]
    async fn absent_values_leave_the_previous_reading_alone() {
        let handle = driver();
        let mut topics = topic_stream(&handle);
        let channel = fix_channel(handle);

        report(&channel, COMPLETE);
        let flying = instruments_at(&mut topics, 50.823).await;

        report(&channel, POSITION_ONLY);
        let landed = instruments_at(&mut topics, 51.0).await;

        assert_eq!(landed.track_degrees, flying.track_degrees);
        assert_eq!(
            landed.ground_speed_meters_per_second,
            flying.ground_speed_meters_per_second
        );
        assert_eq!(landed.altitude_msl_meters, flying.altitude_msl_meters);
    }

    #[tokio::test]
    async fn back_to_back_reported_fixes_preserve_channel_order() {
        let handle = driver();
        let mut topics = topic_stream(&handle);
        let channel = fix_channel(handle);

        report(&channel, COMPLETE);
        report(&channel, POSITION_ONLY);

        let first = instruments_at(&mut topics, 50.823).await;
        let second = instruments_at(&mut topics, 51.0).await;

        assert_eq!(assert_some!(first.position).latitude_degrees, 50.823);
        assert_eq!(assert_some!(second.position).latitude_degrees, 51.0);
    }

    #[tokio::test]
    #[traced_test]
    async fn payloads_that_are_not_fixes_reach_nothing() {
        let handle = driver();
        let mut topics = topic_stream(&handle);
        let channel = fix_channel(handle);

        let onboarding = timeout(PATIENCE, topics.recv())
            .await
            .expect("the onboarding topic within the timeout");
        assert_some_eq!(onboarding, Topic::Instruments(Instruments::default()));

        report(&channel, "{}");
        report(&channel, COMPLETE);

        // A driver that emitted nothing at all, for any reason, would also
        // pass a bare timeout on the malformed payload. Sending a well-formed
        // fix right behind it and requiring it to be the next instruments topic
        // proves both that the malformed payload reached no topic of its own
        // and that the driver kept running.
        let instruments = next_instruments(&mut topics).await;
        assert_eq!(assert_some!(instruments.position).latitude_degrees, 50.823);

        assert!(logs_contain("Discarded an unreadable GNSS fix"));
    }

    /// A renamed optional field has to fail the fix rather than deserialize to
    /// `None`. Dropped instead, it would leave the instrument it feeds holding
    /// its last reading while the position kept moving, which is the one
    /// failure a pilot cannot see.
    #[tokio::test]
    #[traced_test]
    async fn renamed_optional_field_is_rejected_rather_than_dropped() {
        let handle = driver();
        let mut topics = topic_stream(&handle);
        let channel = fix_channel(handle);

        let onboarding = timeout(PATIENCE, topics.recv())
            .await
            .expect("the onboarding topic within the timeout");
        assert_some_eq!(onboarding, Topic::Instruments(Instruments::default()));

        report(&channel, RENAMED_OPTIONAL);
        report(&channel, COMPLETE);

        // The well-formed fix behind it must be the next instruments topic: anything
        // the renamed payload published would arrive first and carry a track
        // of `None`.
        let instruments = next_instruments(&mut topics).await;
        assert_some_eq!(instruments.track_degrees, 270.0_f64);

        assert!(logs_contain("Discarded an unreadable GNSS fix"));
    }
}
