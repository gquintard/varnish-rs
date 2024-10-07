varnish::run_vtc_tests!("tests/*.vtc");

/// Listen to VCL event
#[varnish::vmod(docs = "README.md")]
mod event {
    use std::sync::atomic::AtomicI64;
    use std::sync::atomic::Ordering::Relaxed;

    use varnish::vcl::{Ctx, Event, LogTag};

    /// number of times on_event() was called with `Event::Load`
    /// `on_event()` will increment it and store it in the `PRIV_VCL`
    /// loaded() will access it and return to the VCL
    static EVENT_LOADED_COUNT: AtomicI64 = AtomicI64::new(0);

    /// Return the number of VCL loads stored during when the event function ran.
    pub fn loaded(#[shared_per_vcl] shared: Option<&i64>) -> i64 {
        shared.copied().unwrap_or(0)
    }

    /// This function is called implicitly when your VCL is loaded, discarded, warmed or cooled.
    /// In this vmod, the event function will prevent the second VCL that imports the vmod from loading.
    /// It will also store the number of time this VCL has been loaded.
    /// See also <https://varnish-cache.org/docs/6.2/reference/vmod.html#event-functions>
    #[event]
    pub fn on_event(
        ctx: &mut Ctx,
        #[shared_per_vcl] shared: &mut Option<Box<i64>>,
        event: Event,
    ) -> Result<(), &'static str> {
        // log the event, showing that it implements Debug
        ctx.log(LogTag::Debug, format!("event: {event:?}"));

        // we only care about load events, which is why we don't use `match`
        if matches!(event, Event::Load) {
            // increment the count in a thread-safe way
            let last_count = EVENT_LOADED_COUNT.fetch_add(1, Relaxed);
            if last_count == 1 {
                // Demo that we can fail on the second `load` event
                return Err("second load always fail");
            }

            // store the count, so it is accessible in the `loaded()` VCL function
            let new_count = last_count + 1;
            match shared {
                None => {
                    // This is the first time we're running this function in the VCL context
                    *shared = Some(Box::new(new_count));
                }
                Some(shared) => {
                    // Update box content in-place to the new value
                    **shared = new_count;
                }
            }
        }

        Ok(())
    }
}
