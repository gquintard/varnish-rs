use std::borrow::Cow;
use std::os::raw::c_uint;
use std::time::Duration;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum COWRequest<'a> {
    URL(Cow<'a, str>),
    Text(Cow<'a, str>),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct COWProbe<'a> {
    pub request: COWRequest<'a>,
    pub timeout: Duration,
    pub interval: Duration,
    pub exp_status: c_uint,
    pub window: c_uint,
    pub threshold: c_uint,
    pub initial: c_uint,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum Request {
    URL(String),
    Text(String),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Probe {
    pub request: Request,
    pub timeout: Duration,
    pub interval: Duration,
    pub exp_status: c_uint,
    pub window: c_uint,
    pub threshold: c_uint,
    pub initial: c_uint,
}

impl<'a> COWProbe<'a> {
    pub fn to_owned(&self) -> Probe {
        Probe {
            request: match &self.request {
                COWRequest::URL(cow) => Request::URL(cow.to_string()),
                COWRequest::Text(cow) => Request::Text(cow.to_string()),
            },
            timeout: self.timeout,
            interval: self.interval,
            exp_status: self.exp_status,
            window: self.window,
            threshold: self.threshold,
            initial: self.initial,
        }
    }
}
