#![deny(unused_features)]
#![deny(deprecated)]
#![warn(unused_variables)]
#![warn(unused_imports)]
#![warn(dead_code)]
#![warn(missing_copy_implementations)]

#[macro_use]
extern crate log;

pub mod helper;
mod middleware;
mod requests;
mod server;

pub use actix_web::{
    error::PayloadError, http::header::HeaderMap, web::Payload, HttpMessage, HttpRequest,
    HttpResponse,
};
pub use requests::{Request, RequestReceiver};
pub use server::{new, TestServer};
