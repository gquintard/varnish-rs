#[varnish::stats]
pub struct Stats {
    #[counter(oneliner = "A counter")]
    hits: std::sync::atomic::AtomicU64,

    #[gauge(oneliner = "Another counter")]
    temperature: std::sync::atomic::AtomicU64,
}

#[allow(non_camel_case_types)]
pub struct test {
    stats: Stats,
}

#[varnish::vmod(docs = "README.md")]
mod stats {
    use super::{test, Stats, VscCounterStruct};

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
    }
}

#[cfg(test)]
mod tests {
    // run all VTC tests
    varnish::run_vtc_tests!("tests/*.vtc");
}
