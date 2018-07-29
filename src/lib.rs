extern crate actix;
extern crate actix_web;
extern crate antidote;
extern crate bytes;
extern crate crossbeam_channel as channel;
extern crate futures;
#[macro_use]
extern crate lazy_static;
extern crate rand;

pub use self::server::TestServer;
pub use actix_web::{HttpRequest, HttpResponse};

mod middlware;
mod server;

use antidote::Mutex;
use std::collections::HashMap;

lazy_static! {
    pub(crate) static ref MAP: Mutex<HashMap<u8, Request>> = Mutex::new(HashMap::new());
}

#[derive(Debug)]
pub struct Request {
    pub body: String,
    pub headers: HashMap<String, String>,
    pub method: String,
    pub path: String,
}

pub(crate) struct SendRequest {
    tx: channel::Sender<u8>,
}
