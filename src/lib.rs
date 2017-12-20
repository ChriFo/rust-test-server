#![allow(dead_code)]
extern crate spmc;
pub extern crate futures;
pub extern crate hyper;

use self::futures::Future;
use self::futures::sync::oneshot;
use self::hyper::server::{Http, NewService, Request, Response, Service};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

pub struct Serve {
    addr: SocketAddr,
    msg_rx: mpsc::Receiver<Msg>,
    reply_tx: spmc::Sender<Reply>,
    shutdown_signal: Option<oneshot::Sender<()>>,
    thread: Option<thread::JoinHandle<()>>,
}

impl Serve {
    pub fn addr(&self) -> &SocketAddr {
        &self.addr
    }

    pub fn request(&self) -> Option<Request> {
        match self.msg_rx.try_recv() {
            Ok(Msg::Request(request)) => Some(request),
            _ => None,
        }
    }

    pub fn reply(&self) -> ReplyBuilder {
        ReplyBuilder { tx: &self.reply_tx }
    }
}

pub struct ReplyBuilder<'a> {
    tx: &'a spmc::Sender<Reply>,
}

impl<'a> ReplyBuilder<'a> {
    pub fn status(self, status: hyper::StatusCode) -> Self {
        self.tx.send(Reply::Status(status)).unwrap();
        self
    }

    pub fn header<H: hyper::header::Header>(self, header: H) -> Self {
        let mut headers = hyper::Headers::new();
        headers.set(header);
        self.tx.send(Reply::Headers(headers)).unwrap();
        self
    }

    pub fn body<T: AsRef<[u8]>>(self, body: T) {
        self.tx.send(Reply::Body(body.as_ref().into())).unwrap();
    }
}

impl Drop for Serve {
    fn drop(&mut self) {
        drop(self.shutdown_signal.take());
        self.thread.take().unwrap().join().unwrap();
    }
}

#[derive(Clone)]
struct TestService {
    tx: Arc<Mutex<mpsc::Sender<Msg>>>,
    reply: spmc::Receiver<Reply>,
    _timeout: Option<Duration>,
}

#[derive(Clone, Debug)]
enum Reply {
    Status(hyper::StatusCode),
    Headers(hyper::Headers),
    Body(Vec<u8>),
}

enum Msg {
    Request(Request),
}

impl NewService for TestService {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;

    type Instance = TestService;

    fn new_service(&self) -> ::std::io::Result<TestService> {
        Ok(self.clone())
    }
}

impl Service for TestService {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item = Response, Error = hyper::Error>>;
    fn call(&self, req: Request) -> Self::Future {
        let tx = self.tx.clone();
        let replies = self.reply.clone();

        let _ = tx.lock().unwrap().send(Msg::Request(req));

        let mut res = Response::new();
        while let Ok(reply) = replies.try_recv() {
            match reply {
                Reply::Status(s) => {
                    res.set_status(s);
                }
                Reply::Headers(headers) => {
                    *res.headers_mut() = headers;
                }
                Reply::Body(body) => {
                    res.set_body(body);
                }
            }
        }

        Box::new(futures::future::ok(res))
    }
}

pub fn serve(addr: Option<String>) -> Serve {
    let (addr_tx, addr_rx) = mpsc::channel();
    let (msg_tx, msg_rx) = mpsc::channel();
    let (reply_tx, reply_rx) = spmc::channel();
    let (shutdown_tx, shutdown_rx) = oneshot::channel();

    let addr = match addr {
        Some(addr) => addr.parse().unwrap(),
        None => "127.0.0.1:0".parse().unwrap()
    };

    let thread = thread::Builder::new()
        .spawn(move || {
            let srv = Http::new()
                .pipeline(false)
                .bind(
                    &addr,
                    TestService {
                        tx: Arc::new(Mutex::new(msg_tx.clone())),
                        _timeout: None,
                        reply: reply_rx,
                    },
                )
                .unwrap();
            addr_tx.send(srv.local_addr().unwrap()).unwrap();
            srv.run_until(shutdown_rx.then(|_| Ok(()))).unwrap();
        })
        .unwrap();

    let addr = addr_rx.recv().unwrap();

    Serve {
        msg_rx: msg_rx,
        reply_tx: reply_tx,
        addr: addr,
        shutdown_signal: Some(shutdown_tx),
        thread: Some(thread),
    }
}
