#![allow(dead_code)]
extern crate actix;
pub extern crate actix_web;
extern crate crossbeam_channel as channel;
extern crate futures;

use actix::prelude::{Addr, Syn, System};
use actix_web::middleware::{Middleware, Started};
use actix_web::server::{self, HttpHandler, HttpServer};
use actix_web::{App, HttpMessage, HttpRequest, HttpResponse, Result};
use futures::Future;
use std::collections::HashMap;
use std::io::Read;
use std::net::SocketAddr;
use std::thread;

#[derive(Debug)]
pub struct SendedRequest {
    body: String,
    headers: HashMap<String, String>,
    method: String,
    path: String,
}

impl<S> From<HttpRequest<S>> for SendedRequest {
    fn from(req: HttpRequest<S>) -> Self {
        let mut request = req.clone();

        // https://github.com/actix/actix-web/issues/373
        let mut body = String::new();
        let _ = request.read_to_string(&mut body);

        let headers = request
            .headers()
            .iter()
            .map(|(k, v)| {
                (
                    k.as_str().to_string(),
                    v.to_str()
                        .expect("Failed to convert header value")
                        .to_string(),
                )
            })
            .collect::<HashMap<_, _>>();

        let method = request.method().to_string();
        let path = request.path().to_string();

        SendedRequest {
            body,
            headers,
            method,
            path,
        }
    }
}

struct SendRequest {
    tx: channel::Sender<SendedRequest>,
}

impl<S: 'static> Middleware<S> for SendRequest {
    fn start(&self, req: &mut HttpRequest<S>) -> Result<Started> {
        let request: SendedRequest = req.clone().into();

        self.tx.send(request);

        Ok(Started::Done)
    }
}

pub struct TestServer {
    addr: Addr<Syn, HttpServer<Box<HttpHandler>>>,
    rx_req: channel::Receiver<SendedRequest>,
    socket: SocketAddr,
}

impl TestServer {
    pub fn new(port: u16, func: fn(HttpRequest) -> HttpResponse) -> Self {
        let (tx, rx) = channel::unbounded();
        let (tx_req, rx_req) = channel::unbounded();

        let _ = thread::spawn(move || {
            let sys = System::new("test-server");
            let server = server::new(move || {
                vec![
                    App::new()
                        .middleware(SendRequest { tx: tx_req.clone() })
                        .default_resource(move |r| r.f(func))
                        .boxed(),
                ]
            }).bind(SocketAddr::from(([127, 0, 0, 1], port)))
                .expect("Failed to bind");

            let socket = server.addrs()[0];
            let addr = server.shutdown_timeout(0).start();
            let _ = tx.clone().send((addr, socket));
            let _ = sys.run();
        });

        let (addr, socket) = rx.recv().expect("Failed to receive instance addr");

        Self {
            addr,
            rx_req,
            socket,
        }
    }

    pub fn received_request(&self) -> Option<SendedRequest> {
        self.rx_req.try_recv()
    }

    pub fn url(&self) -> String {
        format!("http://{}:{}", self.socket.ip(), self.socket.port())
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        let _ = self.addr
            .send(server::StopServer { graceful: false })
            .wait();
    }
}

#[cfg(test)]
mod tests {

    extern crate rand;
    extern crate reqwest;

    use self::rand::{distributions::Alphanumeric, Rng};
    use self::reqwest::StatusCode;
    use super::actix_web::HttpResponse;
    use super::{SendedRequest, TestServer};
    use std::{fs::File, io::Read};

    #[test]
    fn start_server_at_given_port() {
        let server = TestServer::new(65432, |_| HttpResponse::Ok().into());

        assert!(&server.url().contains(":65432"));

        let response = reqwest::get(&server.url()).unwrap();

        assert_eq!(StatusCode::Ok, response.status());
    }

    #[test]
    fn validate_client_request() {
        let server = TestServer::new(0, |_| HttpResponse::Ok().into());

        let request_content = create_rand_string(100);
        let client = reqwest::Client::new();
        let _ = client
            .post(&server.url())
            .body(request_content.clone())
            .send();

        let request = server.received_request();
        assert!(request.is_some());

        #[allow(unused_variables)]
        let SendedRequest {
            body, // https://github.com/actix/actix-web/issues/373
            headers,
            method,
            path,
        } = request.unwrap();

        //assert_eq!(request_content, body);
        assert_eq!(Some(&String::from("100")), headers.get("content-length"));
        assert_eq!("POST", method);
        assert_eq!("/", path);
    }

    #[test]
    fn not_necessary_to_fetch_request_from_server() {
        let server = TestServer::new(0, |_| {
            let content = read_file("tests/sample.json");
            HttpResponse::Ok().body(content).into()
        });
        let mut response = reqwest::get(&server.url()).unwrap();

        assert_eq!(read_file("tests/sample.json"), response.text().unwrap());
    }

    fn create_rand_string(size: usize) -> String {
        rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(size)
            .collect::<String>()
    }

    fn read_file(file: &str) -> String {
        let mut file = File::open(file).unwrap();
        let mut content = String::new();
        let _ = file.read_to_string(&mut content);
        content
    }
}
