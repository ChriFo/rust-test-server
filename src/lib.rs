extern crate actix_web;
extern crate bytes;
extern crate crossbeam_channel as channel;
extern crate futures;
extern crate rand;

pub use actix_web::{HttpRequest, HttpResponse};
pub use rand::random;
pub use requests::{Request, RequestReceiver};
pub use server::TestServer;

pub mod helper;
mod middleware;
mod requests;
mod server;
