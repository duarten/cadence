// Cadence - An extensible Statsd client for Rust!
//
// Copyright 2020 Nick Pillitteri
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::builder::MetricBuilder;
use crate::client::{Counted, Gauged, Histogrammed, Metered, StatsdClient, Timed};
use crate::types::{Counter, Gauge, Histogram, Meter, MetricResult, Timer};
use std::time::Duration;

/// Backwards compatibility shim for removed and deprecated methods.
///
/// Due to the way the various metric traits are implemented (`Counted`, `Timed`,
/// etc.), they cannot contain any methods that don't take a metric value as
/// an argument (Rust can't tell what the type `T` for the metric value is in
/// this case).
///
/// To allow people time to migrate, the removed methods (either due to the
/// limitation above or just deprecation) are implemented here. These methods
/// should be considered deprecated and not viable to use long-term (this trait
/// will be removed in a future release).
pub trait Compat {
    #[deprecated(note = "Use `client.count(key, 1)`")]
    #[allow(deprecated)]
    fn incr(&self, key: &str) -> MetricResult<Counter> {
        self.incr_with_tags(key).try_send()
    }

    #[deprecated(note = "Use `client.count_with_tags(key, 1)`")]
    fn incr_with_tags<'a>(&'a self, key: &'a str) -> MetricBuilder<'_, '_, Counter>;

    #[deprecated(note = "Use `client.count(key, -1)`")]
    #[allow(deprecated)]
    fn decr(&self, key: &str) -> MetricResult<Counter> {
        self.decr_with_tags(key).try_send()
    }

    #[deprecated(note = "Use `client.count_with_tags(key, -1)`")]
    fn decr_with_tags<'a>(&'a self, key: &'a str) -> MetricBuilder<'_, '_, Counter>;

    #[deprecated(note = "Use `client.time(key, val)`")]
    #[allow(deprecated)]
    fn time_duration(&self, key: &str, val: Duration) -> MetricResult<Timer> {
        self.time_duration_with_tags(key, val).try_send()
    }

    #[deprecated(note = "Use `client.time_with_tags(key, val)`")]
    fn time_duration_with_tags<'a>(&'a self, key: &'a str, val: Duration) -> MetricBuilder<'_, '_, Timer>;

    #[deprecated(note = "Use `client.gauge(key, val)`")]
    #[allow(deprecated)]
    fn gauge_f64(&self, key: &str, val: f64) -> MetricResult<Gauge> {
        self.gauge_f64_with_tags(key, val).try_send()
    }

    #[deprecated(note = "Use `client.gauge_with_tags(key, val)`")]
    fn gauge_f64_with_tags<'a>(&'a self, key: &'a str, val: f64) -> MetricBuilder<'_, '_, Gauge>;

    #[deprecated(note = "Use `client.meter(key, 1)`")]
    #[allow(deprecated)]
    fn mark(&self, key: &str) -> MetricResult<Meter> {
        self.mark_with_tags(key).try_send()
    }

    #[deprecated(note = "Use `client.meter_with_tags(key, 1)`")]
    fn mark_with_tags<'a>(&'a self, key: &'a str) -> MetricBuilder<'_, '_, Meter>;

    #[deprecated(note = "Use `client.histogram(key, val)`")]
    #[allow(deprecated)]
    fn histogram_duration(&self, key: &str, val: Duration) -> MetricResult<Histogram> {
        self.histogram_duration_with_tags(key, val).try_send()
    }
    #[deprecated(note = "Use `client.histogram_with_tags(key, val)`")]
    fn histogram_duration_with_tags<'a>(&'a self, key: &'a str, val: Duration) -> MetricBuilder<'_, '_, Histogram>;
}

impl Compat for StatsdClient {
    fn incr_with_tags<'a>(&'a self, key: &'a str) -> MetricBuilder<'_, '_, Counter> {
        self.count_with_tags(key, 1i64)
    }

    fn decr_with_tags<'a>(&'a self, key: &'a str) -> MetricBuilder<'_, '_, Counter> {
        self.count_with_tags(key, -1i64)
    }

    fn time_duration_with_tags<'a>(&'a self, key: &'a str, val: Duration) -> MetricBuilder<'_, '_, Timer> {
        self.time_with_tags(key, val)
    }

    fn gauge_f64_with_tags<'a>(&'a self, key: &'a str, val: f64) -> MetricBuilder<'_, '_, Gauge> {
        self.gauge_with_tags(key, val)
    }

    fn mark_with_tags<'a>(&'a self, key: &'a str) -> MetricBuilder<'_, '_, Meter> {
        self.meter_with_tags(key, 1)
    }

    fn histogram_duration_with_tags<'a>(&'a self, key: &'a str, val: Duration) -> MetricBuilder<'_, '_, Histogram> {
        self.histogram_with_tags(key, val)
    }
}

#[cfg(test)]
mod tests {
    #![allow(deprecated)]

    use std::time::Duration;

    use crate::client::StatsdClient;
    use crate::sinks::NopMetricSink;

    use super::Compat;

    #[test]
    fn test_statsd_client_counter_compat_methods() {
        let client = StatsdClient::from_sink("test.prefix", NopMetricSink);

        client.incr("some.counter").unwrap();
        client.incr_with_tags("some.counter").try_send().unwrap();
        client.decr("some.counter").unwrap();
        client.decr_with_tags("some.counter").try_send().unwrap();
    }

    #[test]
    fn test_statsd_client_timer_compat_methods() {
        let client = StatsdClient::from_sink("test.prefix", NopMetricSink);

        client.time_duration("some.timer", Duration::from_millis(123)).unwrap();
        client
            .time_duration_with_tags("some.timer", Duration::from_millis(123))
            .try_send()
            .unwrap();
    }

    #[test]
    fn test_statsd_client_gauge_compat_methods() {
        let client = StatsdClient::from_sink("test.prefix", NopMetricSink);

        client.gauge_f64("some.gauge", 4.9).unwrap();
        client.gauge_f64_with_tags("some.gauge", 4.9).try_send().unwrap();
    }

    #[test]
    fn test_statsd_client_meter_compat_methods() {
        let client = StatsdClient::from_sink("test.prefix", NopMetricSink);

        client.mark("some.meter").unwrap();
        client.mark_with_tags("some.meter").try_send().unwrap();
    }

    #[test]
    fn test_statsd_client_histogram_compat_methods() {
        let client = StatsdClient::from_sink("test.prefix", NopMetricSink);

        client
            .histogram_duration("some.histogram", Duration::from_nanos(4433))
            .unwrap();
        client
            .histogram_duration_with_tags("some.histogram", Duration::from_nanos(4433))
            .try_send()
            .unwrap();
    }
}
