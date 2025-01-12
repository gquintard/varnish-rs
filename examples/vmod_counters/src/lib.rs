#[varnish::stats]
pub struct Stats {
    /// Number of hits
    #[counter]
    hits: std::sync::atomic::AtomicU64,

    /// Temperature in degrees Celsius
    #[gauge]
    temperature: std::sync::atomic::AtomicU64,

    /// Memory usage in bytes
    ///
    /// Memory usage can vary quite a bit, based on the number of foo objects.
    #[gauge(level = "debug", format = "bytes")]
    memory: std::sync::atomic::AtomicU64,
}

#[allow(non_camel_case_types)]
pub struct test {
    stats: Stats,
}

#[varnish::vmod(docs = "README.md")]
mod stats {
    use super::{test, Stats};
    use varnish::vsc_types::VscCounterStruct;

    impl test {
        pub fn new() -> Self {
            let stats = Stats::new("mystats", "default");

            Self { stats }
        }

        pub fn increment_hits(&self) {
            self.stats
                .hits
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }

        pub fn get_hits(&self) -> i64 {
            self.stats
                .hits
                .load(std::sync::atomic::Ordering::Relaxed)
                .try_into()
                .unwrap()
        }

        pub fn update_temperature(&self, value: i64) {
            self.stats.temperature.store(
                value.try_into().unwrap(),
                std::sync::atomic::Ordering::Relaxed,
            );
        }

        pub fn get_temperature(&self) -> i64 {
            self.stats
                .temperature
                .load(std::sync::atomic::Ordering::Relaxed)
                .try_into()
                .unwrap()
        }

        pub fn get_memory(&self) -> i64 {
            self.stats
                .memory
                .load(std::sync::atomic::Ordering::Relaxed)
                .try_into()
                .unwrap()
        }

        pub fn update_memory(&self, value: i64) {
            self.stats.memory.store(
                value.try_into().unwrap(),
                std::sync::atomic::Ordering::Relaxed,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    // run all VTC tests
    varnish::run_vtc_tests!("tests/*.vtc");
}
