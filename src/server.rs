use crate::requests::{RequestReceiver, ShareRequest};
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Result};
use failure::{format_err, Error};
use futures::Future;
use std::{net::SocketAddr, rc::Rc};

pub struct TestServer {
    instance: Rc<actix_web::dev::Server>,
    pub requests: Rc<RequestReceiver>,
    socket: Rc<SocketAddr>,
}

impl TestServer {
    pub fn stop(&self) {
        let _ = self.instance.stop(true).wait();
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

pub fn new(port: u16, func: fn(req: HttpRequest) -> HttpResponse) -> Result<TestServer, Error> {
    let (tx, rx) = crate::channel::unbounded();
    let (tx_req, rx_req) = crate::channel::unbounded();

    let _ = ::std::thread::spawn(move || {
        let sys = actix_rt::System::new("test-server");

        let server = HttpServer::new(move || {
            App::new()
                .wrap(ShareRequest::new(tx_req.clone()))
                .default_service(web::route().to(func))
        })
        .bind(SocketAddr::from(([127, 0, 0, 1], port)))
        .expect("Failed to bind!");

        let sockets = server.addrs();
        let instance = server.shutdown_timeout(0).start();
        let _ = tx.clone().send((instance, sockets));

        sys.run()
    });

    let (server, sockets) = rx.recv()?;
    let socket = sockets
        .get(0)
        .ok_or_else(|| format_err!("Failed to get socket addr!"))?;

    Ok(TestServer {
        instance: Rc::new(server),
        requests: Rc::new(RequestReceiver {
            rx: Rc::new(rx_req),
        }),
        socket: Rc::new(*socket),
    })
}
