#![allow(dead_code)]
extern crate antidote;
extern crate crossbeam_channel as channel;
pub extern crate iron;
extern crate url;

use antidote::Mutex;
use iron::{middleware::Handler, prelude::*, BeforeMiddleware, Headers, Listening};
use std::{io::Read, sync::mpsc::RecvError};
use url::Url;

pub struct LastRequest {
    body: String,
    headers: Headers,
    method: String,
    path: String,
}

struct SendRequest {
    tx: Mutex<channel::Sender<LastRequest>>,
}

impl BeforeMiddleware for SendRequest {
    fn before(&self, request: &mut Request) -> IronResult<()> {
        let mut body = String::new();
        request
            .body
            .read_to_string(&mut body)
            .expect("Failed to read request body");

        let url: Url = request.url.clone().into();

        let last_request = LastRequest {
            body,
            headers: request.headers.clone(),
            method: request.method.clone().as_ref().to_string(),
            path: url.as_str().to_string(),
        };

        self.tx.lock().send(last_request);

        Ok(())
    }
}

pub struct TestServer {
    instance: Listening,
    rx: channel::Receiver<LastRequest>,
}

impl TestServer {
    pub fn new(port: u16, handler: Box<Handler>) -> Self {
        let (tx, rx) = channel::bounded(1);

        let mut chain = Chain::new(handler);
        chain.link_before(SendRequest { tx: Mutex::new(tx) });

        TestServer {
            instance: Iron::new(chain)
                .http(("127.0.0.1", port))
                .expect("Unable to start server"),
            rx,
        }
    }

    pub fn last_request(&self) -> Result<LastRequest, RecvError> {
        match self.rx.recv() {
            None => Err(RecvError),
            Some(request) => Ok(request),
        }
    }

    pub fn url(&self) -> String {
        format!(
            "http://{}:{}",
            self.instance.socket.ip(),
            self.instance.socket.port()
        )
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.instance.close().expect("Error closing server");
    }
}

#[cfg(test)]
mod tests {

    extern crate rand;
    extern crate reqwest;

    use self::rand::{distributions::Alphanumeric, Rng};
    use self::reqwest::StatusCode;
    use super::*;

    #[test]
    fn start_server_at_given_port() {
        let server = TestServer::new(65432, Box::new(TestHandler {}));

        assert!(&server.url().contains(":65432"));

        let response = reqwest::get(&server.url()).unwrap();

        assert_eq!(StatusCode::Ok, response.status());
    }

    #[test]
    fn validate_client_request() {
        let server = TestServer::new(0, Box::new(TestHandler {}));

        let request_content = create_rand_string(100);
        let client = reqwest::Client::new();
        let _ = client
            .post(&server.url())
            .body(request_content.clone())
            .send();

        let last_request = server.last_request().unwrap();

        assert_eq!(request_content, last_request.body);
    }

    struct TestHandler;

    impl Handler for TestHandler {
        fn handle(&self, _: &mut Request) -> IronResult<Response> {
            Ok(Response::with((iron::status::Ok, "ok")))
        }
    }

    fn create_rand_string(size: usize) -> String {
        rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(size)
            .collect::<String>()
    }
}
