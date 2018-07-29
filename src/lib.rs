extern crate actix;
extern crate actix_web;
extern crate antidote;
extern crate bytes;
extern crate crossbeam_channel as channel;
extern crate futures;
#[macro_use]
extern crate lazy_static;
extern crate rand;
#[cfg(test)]
extern crate reqwest;

use actix::prelude::{Addr, System};
pub use actix_web::HttpRequest;
pub use actix_web::HttpResponse;
use actix_web::{
    middleware::{Middleware, Started}, server::{self, HttpHandler, HttpHandlerTask, HttpServer},
    App, HttpMessage, Result,
};
use antidote::Mutex;
use bytes::BytesMut;
use futures::{Future, Stream};
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
    fn start(&self, req: &HttpRequest<S>) -> Result<Started> {
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

        let fut = req.clone()
            .payload()
            .from_err()
            .fold(
                BytesMut::new(),
                move |mut body, chunk| -> Result<_, actix_web::Error> {
                    body.extend_from_slice(&chunk);
                    Ok(body)
                },
            )
            .and_then(move |body| {
                MAP.lock().insert(
                    id,
                    Request {
                        body: String::from_utf8(body.to_vec())
                            .expect("Failed to extract request body"),
                        headers,
                        method,
                        path,
                    },
                );
                Ok(None)
            });

        Ok(Started::Future(Box::new(fut)))
    }
}

type ServerAddrType = Addr<HttpServer<Box<HttpHandler<Task = Box<HttpHandlerTask>>>>>;

pub struct TestServer {
    addr: ServerAddrType,
    request: channel::Receiver<u8>,
    socket: (IpAddr, u16),
}

impl TestServer {
    pub fn new(port: u16, func: for<'r> fn(&'r HttpRequest) -> HttpResponse) -> Self {
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

    use super::{HttpResponse, Request, TestServer};
    use rand::{self, distributions::Alphanumeric, Rng};
    use reqwest::{self, StatusCode};
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

    #[test]
    fn fetch_2nd_request_from_server() {
        let server = TestServer::new(0, |_| HttpResponse::Ok().into());

        let _ = reqwest::get(&server.url()).unwrap();
        let _ = reqwest::Client::new().post(&server.url()).body("2").send();

        let _ = server.received_request().unwrap();
        let request = server.received_request().unwrap();

        assert_eq!("2", request.body);
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
