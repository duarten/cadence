// Cadence - An extensible Statsd client for Rust!
//
// To the extent possible under law, the author(s) have dedicated all copyright and
// related and neighboring rights to this file to the public domain worldwide.
// This software is distributed without any warranty.
//
// You should have received a copy of the CC0 Public Domain Dedication along with this
// software. If not, see <http://creativecommons.org/publicdomain/zero/1.0/>.

// This example shows how you might make use of the "Spy" sinks in Cadence
// which are meant for integration testing your application. They allow callers
// to retain a reference to an underlying `Write` implementation that can then
// be used to verify that Cadence wrote what you thought it was going to write.

use cadence::prelude::*;
use cadence::{BufferedSpyMetricSink, StatsdClient};

fn main() {
    // Ensure that the sink is dropped, forcing a flush of all buffered metrics.
    let rx = {
        // Use a buffer size larger than any metrics here so we can demonstrate that
        // each metric ends up with a newline (\n) after it.
        let (rx, sink) = BufferedSpyMetricSink::with_capacity(None, Some(64));
        let metrics = StatsdClient::from_sink("example.prefix", sink);

        metrics.count("example.counter", 1).unwrap();
        metrics.gauge("example.gauge", 5).unwrap();
        metrics.time("example.timer", 32).unwrap();
        metrics.histogram("example.histogram", 22).unwrap();
        metrics.meter("example.meter", 8).unwrap();
        metrics.set("example.set", 43).unwrap();
        rx
    };

    let mut buffer = Vec::new();
    while let Ok(v) = rx.try_recv() {
        buffer.extend(v);
    }

    println!("Contents of wrapped buffer:\n{}", String::from_utf8(buffer).unwrap());
}
