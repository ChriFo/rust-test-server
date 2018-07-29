use super::{Request, ShareRequest, QUEUE};
use actix_web::actix::{Addr, System};
use actix_web::server::{self, HttpHandler, HttpHandlerTask, HttpServer};
use actix_web::{App, HttpRequest, HttpResponse};
use channel;
use futures::Future;
use rand::prelude::random;
use std::net::{IpAddr, SocketAddr};
use std::thread;

type AddrType = Addr<HttpServer<Box<HttpHandler<Task = Box<HttpHandlerTask>>>>>;

pub struct TestServer {
    addr: AddrType,
    id: u8,
    socket: (IpAddr, u16),
}

impl TestServer {
    pub fn new(port: u16, func: fn(&HttpRequest) -> HttpResponse) -> Self {
        let (tx, rx) = channel::unbounded();
        let id: u8 = random();

        let _ = thread::spawn(move || {
            let sys = System::new(format!("test-server-{}", id));
            let server = server::new(move || {
                vec![
                    App::new()
                        .middleware(ShareRequest { id })
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
            id,
            socket: (socket.ip(), socket.port()),
        }
    }

    pub fn requests(&self) -> Vec<Request> {
        match QUEUE.lock().remove(&self.id) {
            Some(requests) => requests,
            None => vec![],
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
