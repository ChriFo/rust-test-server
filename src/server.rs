use actix_net::server::Server;
use actix_web::actix::{Addr, System};
use actix_web::server::{self, StopServer};
use actix_web::{App, HttpRequest, HttpResponse};
use futures::Future;
use requests::{RequestReceiver, ShareRequest};
use std::net::{IpAddr, SocketAddr};

pub struct TestServer {
    addr: Addr<Server>,
    pub requests: RequestReceiver,
    socket: (IpAddr, u16),
}

impl TestServer {
    pub fn stop(&self) {
        let _ = self.addr.send(StopServer { graceful: true }).wait();
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

pub fn new(port: u16, func: fn(&HttpRequest) -> HttpResponse) -> TestServer {
    let (tx, rx) = ::channel::unbounded();
    let (tx_req, rx_req) = ::channel::unbounded();

    let _ = ::std::thread::spawn(move || {
        let sys = System::new("test-server");
        let server = server::new(move || {
            vec![
                App::new()
                    .middleware(ShareRequest { tx: tx_req.clone() })
                    .default_resource(move |r| r.f(func))
                    .boxed(),
            ]
        })
        .bind(SocketAddr::from(([127, 0, 0, 1], port)))
        .expect("Failed to bind");

        let sockets = server.addrs();
        let addr = server.shutdown_timeout(0).start();
        tx.clone().send((addr, sockets));

        let _ = sys.run();
    });

    let (addr, sockets) = rx.recv().expect("Failed to receive instance addr");
    let socket = sockets.get(0).expect("Failed to get bound socket");

    TestServer {
        addr,
        requests: RequestReceiver { rx: rx_req },
        socket: (socket.ip(), socket.port()),
    }
}
