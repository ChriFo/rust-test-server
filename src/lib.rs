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

pub mod helper;
mod middlware;
mod server;

use antidote::Mutex;
use std::collections::HashMap;
use std::vec::Vec;

lazy_static! {
    pub(crate) static ref QUEUE: Mutex<HashMap<u8, Vec<Request>>> = Mutex::new(HashMap::new());
}

#[derive(Debug)]
pub struct Request {
    pub body: String,
    pub headers: HashMap<String, String>,
    pub method: String,
    pub path: String,
}

pub(crate) struct ShareRequest {
    id: u8,
}
