use std::time::Instant;

use tracing::debug;

pub(crate) struct Timing {
    start: Instant,
    identifier: &'static str,
}

impl Timing {
    pub(crate) fn new(identifier: &'static str) -> Self {
        Timing {
            start: Instant::now(),
            identifier,
        }
    }
}

impl Drop for Timing {
    fn drop(&mut self) {
        let elapsed = self.start.elapsed().as_millis();
        debug!("{}, elapsed: {}ms", self.identifier, elapsed);
    }
}
