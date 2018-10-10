extern crate actix_net;
extern crate actix_web;
extern crate bytes;
extern crate crossbeam_channel as channel;
extern crate futures;
extern crate rand;
#[cfg(test)]
extern crate spectral;

pub use actix_web::{HttpRequest, HttpResponse};
pub use requests::{Request, RequestReceiver};
pub use server::{new, TestServer};

pub mod helper;
mod middleware;
mod requests;
mod server;
