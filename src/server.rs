use crate::requests::{RequestReceiver, ShareRequest};
use actix_web::{dev::Factory, web, App, FromRequest, HttpServer, Responder, Result};
use failure::{format_err, Error};
use futures::{executor::block_on, Future};
use std::{
    net::{SocketAddr, ToSocketAddrs},
    rc::Rc,
};

pub struct TestServer {
    instance: Rc<actix_web::dev::Server>,
    pub requests: Rc<RequestReceiver>,
    socket: Rc<SocketAddr>,
}

impl TestServer {
    pub fn stop(&self) {
        block_on(self.instance.stop(false));
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

pub fn new<A, F, T, R, U>(addr: A, func: F) -> Result<TestServer, Error>
where
    A: ToSocketAddrs + 'static + Send + Copy,
    F: Factory<T, R, U> + 'static + Send + Copy,
    T: FromRequest + 'static,
    R: Future<Output = U> + 'static,
    U: Responder + 'static,
{
    let (tx, rx) = crossbeam::channel::unbounded();
    let (tx_req, rx_req) = crossbeam::channel::unbounded();

    let _ = ::std::thread::spawn(move || {
        let sys = actix_rt::System::new("test-server");
        let server = HttpServer::new(move || {
            App::new()
                .wrap(ShareRequest::new(tx_req.clone()))
                .default_service(web::route().to(func))
        })
        .bind(addr)
        .expect("Failed to bind!");

        let sockets = server.addrs();
        let instance = server.shutdown_timeout(1).start();
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
