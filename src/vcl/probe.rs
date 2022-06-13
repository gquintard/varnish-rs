use std::os::raw::c_uint;
use std::borrow::Cow;
use std::time::Duration;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum Request<'a> {
    URL(Cow<'a, str>),
    Text(Cow<'a, str>),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Probe<'a> {
    pub request: Request<'a>,
    pub timeout: Duration,
    pub interval: Duration,
    pub exp_status: c_uint,
    pub window: c_uint,
    pub threshold: c_uint,
    pub initial: c_uint,
}
