// Cadence - An extensible Statsd client for Rust!
//
// Copyright 2018 Philip Jenvey <pjenvey@mozilla.com>
// Copyright 2018-2021 Nick Pillitteri
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::client::{MetricBackend, StatsdClient};
use crate::types::{Metric, MetricError, MetricResult};
use std::fmt::{self, Write};
use std::marker::PhantomData;

/// Uniform holder for values that knows how to display itself
#[derive(Debug, Clone, Copy)]
enum MetricValue {
    Signed(i64),
    Unsigned(u64),
    Float(f64),
}

impl fmt::Display for MetricValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            MetricValue::Signed(i) => i.fmt(f),
            MetricValue::Unsigned(i) => i.fmt(f),
            MetricValue::Float(i) => i.fmt(f),
        }
    }
}

/// Type of metric that knows how to display itself
#[derive(Debug, Clone, Copy)]
enum MetricType {
    Counter,
    Timer,
    Gauge,
    Meter,
    Histogram,
    Set,
}

impl fmt::Display for MetricType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            MetricType::Counter => "c".fmt(f),
            MetricType::Timer => "ms".fmt(f),
            MetricType::Gauge => "g".fmt(f),
            MetricType::Meter => "m".fmt(f),
            MetricType::Histogram => "h".fmt(f),
            MetricType::Set => "s".fmt(f),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct MetricFormatter<'a, T>
where
    T: Metric + From<String>,
{
    metric: PhantomData<T>,
    prefix: &'a str,
    key: &'a str,
    val: MetricValue,
    type_: MetricType,
    tags: Vec<(Option<&'a str>, &'a str)>,
}

impl<'a, T> MetricFormatter<'a, T>
where
    T: Metric + From<String>,
{
    const TAG_PREFIX: &'static str = "|#";

    pub(crate) fn counter(prefix: &'a str, key: &'a str, val: i64) -> Self {
        Self::from_i64(prefix, key, val, MetricType::Counter)
    }

    pub(crate) fn timer(prefix: &'a str, key: &'a str, val: u64) -> Self {
        Self::from_u64(prefix, key, val, MetricType::Timer)
    }

    pub(crate) fn gauge(prefix: &'a str, key: &'a str, val: u64) -> Self {
        Self::from_u64(prefix, key, val, MetricType::Gauge)
    }

    pub(crate) fn gauge_f64(prefix: &'a str, key: &'a str, val: f64) -> Self {
        Self::from_f64(prefix, key, val, MetricType::Gauge)
    }

    pub(crate) fn meter(prefix: &'a str, key: &'a str, val: u64) -> Self {
        Self::from_u64(prefix, key, val, MetricType::Meter)
    }

    pub(crate) fn histogram(prefix: &'a str, key: &'a str, val: u64) -> Self {
        Self::from_u64(prefix, key, val, MetricType::Histogram)
    }

    pub(crate) fn set(prefix: &'a str, key: &'a str, val: i64) -> Self {
        Self::from_i64(prefix, key, val, MetricType::Set)
    }

    fn from_u64(prefix: &'a str, key: &'a str, val: u64, type_: MetricType) -> Self {
        MetricFormatter {
            prefix,
            key,
            type_,
            val: MetricValue::Unsigned(val),
            metric: PhantomData,
            tags: Vec::new(),
        }
    }

    fn from_i64(prefix: &'a str, key: &'a str, val: i64, type_: MetricType) -> Self {
        MetricFormatter {
            prefix,
            key,
            type_,
            val: MetricValue::Signed(val),
            metric: PhantomData,
            tags: Vec::new(),
        }
    }

    fn from_f64(prefix: &'a str, key: &'a str, val: f64, type_: MetricType) -> Self {
        MetricFormatter {
            prefix,
            key,
            type_,
            val: MetricValue::Float(val),
            metric: PhantomData,
            tags: Vec::new(),
        }
    }

    fn with_tag(&mut self, key: &'a str, value: &'a str) {
        self.tags.push((Some(key), value));
    }

    fn with_tag_value(&mut self, value: &'a str) {
        self.tags.push((None, value));
    }

    fn write_base_metric(&self, out: &mut String) {
        let _ = write!(out, "{}{}:{}|{}", self.prefix, self.key, self.val, self.type_);
    }

    fn write_tags(&self, out: &mut String) {
        if !self.tags.is_empty() {
            out.push_str(Self::TAG_PREFIX);
            for (i, &(key, value)) in self.tags.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                if let Some(key) = key {
                    out.push_str(key);
                    out.push(':');
                }
                out.push_str(value);
            }
        }
    }

    // Don't have rustfmt do anything to this method because it keeps wrapping
    // the line when we don't want it to.
    #[rustfmt::skip]
    fn base_metric_size_hint(&self) -> usize {
        // Note: This isn't actually the number of bytes required, it's just
        // a guess. This is probably sufficient in most cases and guessing is
        // faster than actually doing the math to find the exact number of bytes
        // required.
        //
        // Justification for "10" bytes: the max number of digits we could possibly
        // need for the string representation of our value is 20 (for both u64::MAX
        // and i64::MIN including the minus sign). So, 10 digits covers a pretty
        // large range of values that will actually be seen in practice. Plus, using
        // a constant is faster than computing the `val.log(10)` of our value which
        // we would need to know exactly how many digits it takes up.
        self.prefix.len() + self.key.len() + 1 /* : */ + 10 /* value */ + 1 /* | */ + 2 /* type */
    }

    fn tag_size_hint(&self) -> usize {
        if self.tags.is_empty() {
            return 0;
        }

        let kv_size: usize = self
            .tags
            .iter()
            .map(|(key, val)| {
                // keys are optional so either include its length and ':' or zero
                key.map(|s| s.len() + 1 /* : */).unwrap_or(0) + val.len()
            })
            .sum();

        // prefix, keys and values, commas
        Self::TAG_PREFIX.len() + kv_size + self.tags.len() - 1
    }

    pub(crate) fn build(&self) -> T {
        let size_hint = self.base_metric_size_hint() + self.tag_size_hint();
        let mut metric_string = String::with_capacity(size_hint);
        self.write_base_metric(&mut metric_string);
        self.write_tags(&mut metric_string);
        T::from(metric_string)
    }
}

/// Internal state of a `MetricBuilder`
///
/// The builder can either be in the process of formatting a metric to send
/// via a client or it can be simply holding on to an error that it will be
/// dealt with when `.try_send()` or `.send()` is finally invoked.
#[derive(Debug)]
enum BuilderRepr<'m, 'c, T>
where
    T: Metric + From<String>,
{
    Success(MetricFormatter<'m, T>, &'c StatsdClient),
    Error(MetricError, &'c StatsdClient),
}

/// Builder for adding tags to in-progress metrics.
///
/// This builder adds tags, key-value pairs or just values, to a metric that
/// was previously constructed by a call to a method on `StatsdClient`. The
/// tags are added to metrics and sent via the client when `MetricBuilder::send()`
/// or `MetricBuilder::try_send()`is invoked. Any errors encountered constructing,
/// validating, or sending the metrics will be propagated and returned when the
/// those methods are finally invoked.
///
/// Currently, only Datadog style tags are supported. For more information on the
/// exact format used, see the
/// [Datadog docs](https://docs.datadoghq.com/developers/dogstatsd/#datagram-format).
///
/// Adding tags to a metric via this builder will typically result in one or more
/// extra heap allocations.
///
/// NOTE: The only way to instantiate an instance of this builder is via methods in
/// in the `StatsdClient` client.
///
/// # Examples
///
/// ## `.try_send()`
///
/// An example of how the metric builder is used with a `StatsdClient` instance
/// is given below.
///
/// ```
/// use cadence::prelude::*;
/// use cadence::{StatsdClient, NopMetricSink, Metric};
///
/// let client = StatsdClient::from_sink("some.prefix", NopMetricSink);
/// let res = client.incr_with_tags("some.key")
///    .with_tag("host", "app11.example.com")
///    .with_tag("segment", "23")
///    .with_tag_value("beta")
///    .try_send();
///
/// assert_eq!(
///     concat!(
///         "some.prefix.some.key:1|c|#",
///         "host:app11.example.com,",
///         "segment:23,",
///         "beta"
///     ),
///     res.unwrap().as_metric_str()
/// );
/// ```
///
/// In this example, two key-value tags and one value tag are added to the
/// metric before it is finally sent to the Statsd server.
///
/// ## `.send()`
///
/// An example of how the metric builder is used with a `StatsdClient` instance
/// when using the "quiet" method is given below.
///
/// ```
/// use cadence::prelude::*;
/// use cadence::{StatsdClient, NopMetricSink, Metric, Counted};
///
/// let client = StatsdClient::builder("some.prefix", NopMetricSink)
///     .with_error_handler(|e| eprintln!("metric error: {}", e))
///     .build();
/// client.count_with_tags("some.key", 1)
///    .with_tag("host", "app11.example.com")
///    .with_tag("segment", "23")
///    .with_tag_value("beta")
///    .send();
/// ```
///
/// Note that nothing is returned from the `.send()` method. Any errors encountered
/// in this case will be passed to the error handler we registered.
#[must_use = "Did you forget to call .send() after adding tags?"]
#[derive(Debug)]
pub struct MetricBuilder<'m, 'c, T>
where
    T: Metric + From<String>,
{
    repr: BuilderRepr<'m, 'c, T>,
}

impl<'m, 'c, T> MetricBuilder<'m, 'c, T>
where
    T: Metric + From<String>,
{
    pub(crate) fn new(formatter: MetricFormatter<'m, T>, client: &'c StatsdClient) -> Self {
        MetricBuilder {
            repr: BuilderRepr::Success(formatter, client),
        }
    }

    pub(crate) fn from_error(err: MetricError, client: &'c StatsdClient) -> Self {
        MetricBuilder {
            repr: BuilderRepr::Error(err, client),
        }
    }

    /// Add a key-value tag to this metric.
    ///
    /// # Example
    ///
    /// ```
    /// use cadence::prelude::*;
    /// use cadence::{StatsdClient, NopMetricSink, Metric};
    ///
    /// let client = StatsdClient::from_sink("some.prefix", NopMetricSink);
    /// let res = client.incr_with_tags("some.key")
    ///    .with_tag("user", "authenticated")
    ///    .try_send();
    ///
    /// assert_eq!(
    ///    "some.prefix.some.key:1|c|#user:authenticated",
    ///    res.unwrap().as_metric_str()
    /// );
    /// ```
    pub fn with_tag(mut self, key: &'m str, value: &'m str) -> Self {
        if let BuilderRepr::Success(ref mut formatter, _) = self.repr {
            formatter.with_tag(key, value);
        }
        self
    }

    /// Add a value tag to this metric.
    ///
    /// # Example
    ///
    /// ```
    /// use cadence::prelude::*;
    /// use cadence::{StatsdClient, NopMetricSink, Metric};
    ///
    /// let client = StatsdClient::from_sink("some.prefix", NopMetricSink);
    /// let res = client.count_with_tags("some.key", 4)
    ///    .with_tag_value("beta-testing")
    ///    .try_send();
    ///
    /// assert_eq!(
    ///    "some.prefix.some.key:4|c|#beta-testing",
    ///    res.unwrap().as_metric_str()
    /// );
    /// ```
    pub fn with_tag_value(mut self, value: &'m str) -> Self {
        if let BuilderRepr::Success(ref mut formatter, _) = self.repr {
            formatter.with_tag_value(value);
        }
        self
    }

    /// Send a metric using the client that created this builder.
    ///
    /// Note that the builder is consumed by this method and thus `.try_send()`
    /// can only be called a single time per builder.
    ///
    /// # Example
    ///
    /// ```
    /// use cadence::prelude::*;
    /// use cadence::{StatsdClient, NopMetricSink, Metric};
    ///
    /// let client = StatsdClient::from_sink("some.prefix", NopMetricSink);
    /// let res = client.gauge_with_tags("some.key", 7)
    ///    .with_tag("test-segment", "12345")
    ///    .try_send();
    ///
    /// assert_eq!(
    ///    "some.prefix.some.key:7|g|#test-segment:12345",
    ///    res.unwrap().as_metric_str()
    /// );
    /// ```
    pub fn try_send(self) -> MetricResult<T> {
        match self.repr {
            BuilderRepr::Error(err, _) => Err(err),
            BuilderRepr::Success(ref formatter, client) => {
                let metric: T = formatter.build();
                client.send_metric(&metric)?;
                Ok(metric)
            }
        }
    }

    /// Send a metric using the client that created this builder, discarding
    /// successful results and invoking a custom handler for error results.
    ///
    /// By default, if no handler is given, a "no-op" handler is used that
    /// simply discards all errors. If this isn't desired, a custom handler
    /// should be supplied when creating a new `StatsdClient` instance.
    ///
    /// Note that the builder is consumed by this method and thus `.send()`
    /// can only be called a single time per builder.
    ///
    /// # Example
    ///
    /// ```
    /// use cadence::prelude::*;
    /// use cadence::{StatsdClient, MetricError, NopMetricSink};
    ///
    /// fn my_handler(err: MetricError) {
    ///     println!("Metric error: {}", err);
    /// }
    ///
    /// let client = StatsdClient::builder("some.prefix", NopMetricSink)
    ///     .with_error_handler(my_handler)
    ///     .build();
    ///
    /// client.gauge_with_tags("some.key", 7)
    ///    .with_tag("region", "us-west-1")
    ///    .send();
    /// ```
    pub fn send(self) {
        match self.repr {
            BuilderRepr::Error(err, client) => client.consume_error(err),
            BuilderRepr::Success(_, client) => {
                if let Err(e) = self.try_send() {
                    client.consume_error(e);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::MetricFormatter;
    use crate::types::{Counter, Gauge, Histogram, Meter, Metric, Set, Timer};

    #[test]
    fn test_metric_formatter_tag_size_hint_no_tags() {
        let fmt: MetricFormatter<'_, Counter> = MetricFormatter::counter("prefix.", "some.key", 1);
        assert_eq!(0, fmt.tag_size_hint());
    }

    #[test]
    fn test_metric_formatter_tag_size_hint_value() {
        let mut fmt: MetricFormatter<'_, Counter> = MetricFormatter::counter("prefix.", "some.key", 1);
        fmt.with_tag_value("test");

        assert_eq!(6, fmt.tag_size_hint());
    }

    #[test]
    fn test_metric_formatter_tag_size_hint_key_value() {
        let mut fmt: MetricFormatter<'_, Counter> = MetricFormatter::counter("prefix.", "some.key", 1);
        fmt.with_tag("host", "web");
        fmt.with_tag("user", "123");

        assert_eq!(19, fmt.tag_size_hint());
    }

    #[test]
    fn test_metric_formatter_counter_no_tags() {
        let fmt = MetricFormatter::counter("prefix.", "some.key", 4);
        let counter: Counter = fmt.build();

        assert_eq!("prefix.some.key:4|c", counter.as_metric_str());
    }

    #[test]
    fn test_metric_formatter_counter_with_tags() {
        let mut fmt = MetricFormatter::counter("prefix.", "some.key", 4);
        fmt.with_tag("host", "app03.example.com");
        fmt.with_tag("bucket", "2");
        fmt.with_tag_value("beta");

        let counter: Counter = fmt.build();

        assert_eq!(
            "prefix.some.key:4|c|#host:app03.example.com,bucket:2,beta",
            counter.as_metric_str()
        );
    }

    #[test]
    fn test_metric_formatter_timer_no_tags() {
        let fmt = MetricFormatter::timer("prefix.", "some.method", 21);
        let timer: Timer = fmt.build();

        assert_eq!("prefix.some.method:21|ms", timer.as_metric_str());
    }

    #[test]
    fn test_metric_formatter_timer_with_tags() {
        let mut fmt = MetricFormatter::timer("prefix.", "some.method", 21);
        fmt.with_tag("app", "metrics");
        fmt.with_tag_value("async");

        let timer: Timer = fmt.build();

        assert_eq!("prefix.some.method:21|ms|#app:metrics,async", timer.as_metric_str());
    }

    #[test]
    fn test_metric_formatter_gauge_no_tags() {
        let fmt = MetricFormatter::gauge("prefix.", "num.failures", 7);
        let gauge: Gauge = fmt.build();

        assert_eq!("prefix.num.failures:7|g", gauge.as_metric_str());
    }

    #[test]
    fn test_metric_formatter_gauge_with_tags() {
        let mut fmt = MetricFormatter::gauge("prefix.", "num.failures", 7);
        fmt.with_tag("window", "300");
        fmt.with_tag_value("best-effort");

        let gauge: Gauge = fmt.build();

        assert_eq!("prefix.num.failures:7|g|#window:300,best-effort", gauge.as_metric_str());
    }

    #[test]
    fn test_metric_formatter_meter_no_tags() {
        let fmt = MetricFormatter::meter("prefix.", "user.logins", 3);
        let meter: Meter = fmt.build();

        assert_eq!("prefix.user.logins:3|m", meter.as_metric_str());
    }

    #[test]
    fn test_metric_formatter_meter_with_tags() {
        let mut fmt = MetricFormatter::meter("prefix.", "user.logins", 3);
        fmt.with_tag("user-type", "verified");
        fmt.with_tag_value("bucket1");

        let meter: Meter = fmt.build();

        assert_eq!(
            "prefix.user.logins:3|m|#user-type:verified,bucket1",
            meter.as_metric_str()
        );
    }

    #[test]
    fn test_metric_formatter_histogram_no_tags() {
        let fmt = MetricFormatter::histogram("prefix.", "num.results", 44);
        let histogram: Histogram = fmt.build();

        assert_eq!("prefix.num.results:44|h", histogram.as_metric_str());
    }

    #[test]
    fn test_metric_formatter_histogram_with_tags() {
        let mut fmt = MetricFormatter::histogram("prefix.", "num.results", 44);
        fmt.with_tag("user-type", "authenticated");
        fmt.with_tag_value("source=search");

        let histogram: Histogram = fmt.build();

        assert_eq!(
            "prefix.num.results:44|h|#user-type:authenticated,source=search",
            histogram.as_metric_str()
        );
    }

    #[test]
    fn test_metric_formatter_set_no_tags() {
        let fmt = MetricFormatter::set("prefix.", "users.uniques", 44);
        let set: Set = fmt.build();

        assert_eq!("prefix.users.uniques:44|s", set.as_metric_str());
    }

    #[test]
    fn test_metric_formatter_set_with_tags() {
        let mut fmt = MetricFormatter::set("prefix.", "users.uniques", 44);
        fmt.with_tag("user-type", "authenticated");
        fmt.with_tag_value("source=search");

        let set: Set = fmt.build();

        assert_eq!(
            concat!(
                "prefix.users.uniques:44|s|#",
                "user-type:authenticated,",
                "source=search"
            ),
            set.as_metric_str()
        );
    }
}
