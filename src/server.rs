use crate::requests::{RequestReceiver, ShareRequest};
use actix_net::server::Server;
use actix_web::{
    actix::{Addr, System},
    server::{self, StopServer},
    App, HttpRequest, HttpResponse,
};
use failure::{format_err, Error};
use futures::Future;
use std::{net::SocketAddr, rc::Rc};

pub struct TestServer {
    addr: Rc<Addr<Server>>,
    pub requests: Rc<RequestReceiver>,
    socket: Rc<SocketAddr>,
}

impl TestServer {
    pub fn stop(&self) {
        let _ = self.addr.send(StopServer { graceful: true }).wait();
    }

    pub fn url(&self) -> String {
        format!("http://{}", self.socket.to_string())
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.stop()
    }
}

pub fn new(port: u16, func: fn(&HttpRequest) -> HttpResponse) -> Result<TestServer, Error> {
    let (tx, rx) = crate::channel::unbounded();
    let (tx_req, rx_req) = crate::channel::unbounded();

    let _ = ::std::thread::spawn(move || {
        let sys = System::new("test-server");
        let server = server::new(move || {
            vec![App::new()
                .middleware(ShareRequest { tx: tx_req.clone() })
                .default_resource(move |r| r.f(func))
                .boxed()]
        })
        .bind(SocketAddr::from(([127, 0, 0, 1], port)))
        .expect("Failed to bind!");

        let sockets = server.addrs();
        let addr = server.shutdown_timeout(0).start();
        let _ = tx.clone().send((addr, sockets));

        let _ = sys.run();
    });

    let (addr, sockets) = rx.recv()?;
    let socket = sockets
        .get(0)
        .ok_or_else(|| format_err!("Failed to get socket addr!"))?;

    Ok(TestServer {
        addr: Rc::new(addr),
        requests: Rc::new(RequestReceiver {
            rx: Rc::new(rx_req),
        }),
        socket: Rc::new(*socket),
    })
}
