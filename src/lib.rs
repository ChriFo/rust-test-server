#![deny(unused_features)]
#![deny(deprecated)]
#![warn(unused_variables)]
#![warn(unused_imports)]
#![warn(dead_code)]
#![warn(missing_copy_implementations)]

#[macro_use]
extern crate log;

pub use crate::requests::{Request, RequestReceiver};
pub use crate::server::{new, TestServer};
pub use actix_http::{error::PayloadError, httpmessage::HttpMessage};
pub use actix_web::{http::header::HeaderMap, web::Payload, HttpRequest, HttpResponse};
use crossbeam::channel;

pub mod helper;
mod middleware;
mod requests;
mod server;
