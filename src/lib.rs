extern crate actix;
extern crate actix_web;
extern crate antidote;
extern crate crossbeam_channel as channel;
extern crate futures;
#[macro_use]
extern crate lazy_static;
extern crate rand;

use actix::prelude::{Addr, Syn, System};
use actix_web::middleware::{Middleware, Started};
use actix_web::server::{self, HttpHandler, HttpServer};
pub use actix_web::HttpRequest;
pub use actix_web::HttpResponse;
use actix_web::{App, HttpMessage, Result};
use antidote::Mutex;
use futures::{future, Future, Stream};
use rand::prelude::random;
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::thread;

lazy_static! {
    static ref MAP: Mutex<HashMap<u8, Request>> = Mutex::new(HashMap::new());
}

#[derive(Debug)]
pub struct Request {
    pub body: String,
    pub headers: HashMap<String, String>,
    pub method: String,
    pub path: String,
}

struct SendRequest {
    tx: channel::Sender<u8>,
}

impl<S: 'static> Middleware<S> for SendRequest {
    fn start(&self, req: &mut HttpRequest<S>) -> Result<Started> {
        let id: u8 = random();
        self.tx.send(id);

        let headers = req.headers()
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

        let method = req.method().to_string();
        let path = req.path().to_string();

        let fut = req.clone().concat2().from_err().and_then(move |body| {
            MAP.lock().insert(
                id,
                Request {
                    body: String::from_utf8(body.to_vec()).expect("Failed to extract request body"),
                    headers,
                    method,
                    path,
                },
            );
            future::ok(None)
        });

        Ok(Started::Future(Box::new(fut)))
    }
}

pub struct TestServer {
    addr: Addr<Syn, HttpServer<Box<HttpHandler>>>,
    request: channel::Receiver<u8>,
    socket: (IpAddr, u16),
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

            let sockets = server.addrs();
            let addr = server.shutdown_timeout(0).start();
            tx.clone().send((addr, sockets));
            let _ = sys.run();
        });

        let (addr, sockets) = rx.recv().expect("Failed to receive instance addr");
        let socket = sockets.get(0).expect("Failed to get bound socket");

        Self {
            addr,
            request: rx_req,
            socket: (socket.ip(), socket.port()),
        }
    }

    pub fn received_request(&self) -> Option<Request> {
        match self.request.try_recv() {
            Some(id) => MAP.lock().remove(&id),
            None => None,
        }
    }

    pub fn stop(&self) {
        let _ = self.addr.send(server::StopServer { graceful: true }).wait();
    }

    pub fn url(&self) -> String {
        format!("http://{}:{}", self.socket.0, self.socket.1)
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.stop()
    }
}

#[cfg(test)]
mod tests {

    extern crate rand;
    extern crate reqwest;

    use self::rand::{distributions::Alphanumeric, Rng};
    use self::reqwest::StatusCode;
    use super::{HttpResponse, Request, TestServer};
    use std::{fs::File, io::Read};

    #[test]
    fn start_server_at_given_port() {
        let server = TestServer::new(65432, |_| HttpResponse::Ok().into());

        assert!(&server.url().contains(":65432"));

        let response = reqwest::get(&server.url()).unwrap();

        assert_eq!(StatusCode::Ok, response.status());
    }

    #[test]
    #[cfg(not(target_os = "windows"))] // carllerche/mio#776
    fn restart_server_at_same_port() {
        let mut server = TestServer::new(65433, |_| HttpResponse::Ok().into());
        let response = reqwest::get(&server.url()).unwrap();

        assert_eq!(StatusCode::Ok, response.status());

        server.stop();
        server = TestServer::new(65433, |_| HttpResponse::BadRequest().into());
        let response = reqwest::get(&server.url()).unwrap();

        assert_eq!(StatusCode::BadRequest, response.status());
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
        let Request {
            body,
            headers,
            method,
            path,
        } = request.unwrap();

        assert_eq!(request_content, body);
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
