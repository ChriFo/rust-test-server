use crate::channel::Sender;
use crate::requests::{Request, ShareRequest};
use actix_http::{error::PayloadError, httpmessage::HttpMessage};
use actix_service::{Service, Transform};
use actix_web::{
    dev::{ServiceRequest, ServiceResponse},
    http::header::HeaderMap,
    Error,
};
use futures::{
    future::{ok, FutureResult},
    Future, Poll, Stream,
};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

impl<S: 'static, B> Transform<S> for ShareRequest
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = ShareRequestMiddleware<S>;
    type Future = FutureResult<Self::Transform, Self::InitError>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(ShareRequestMiddleware {
            service: Rc::new(RefCell::new(service)),
            tx: self.tx.clone(),
        })
    }
}

pub struct ShareRequestMiddleware<S> {
    service: Rc<RefCell<S>>,
    tx: Sender<Request>,
}

impl<S, B> Service for ShareRequestMiddleware<S>
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Box<dyn Future<Item = Self::Response, Error = Self::Error>>;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        self.service.poll_ready()
    }

    fn call(&mut self, mut req: ServiceRequest) -> Self::Future {
        let mut svc = self.service.clone();
        let tx = self.tx.clone();

        let headers = extract_headers(req.headers());
        let query = extract_query(req.query_string());

        let method = req.method().to_string();
        let path = req.path().to_string();

        Box::new(
            req.take_payload()
                .fold(bytes::BytesMut::new(), move |mut body, chunk| {
                    body.extend_from_slice(&chunk);
                    Ok::<_, PayloadError>(body)
                })
                .map_err(|e| e.into())
                .and_then(move |bytes| {
                    let body = bytes.freeze();
                    let _ = tx.send(Request {
                        body: String::from_utf8_lossy(&body.to_vec()).to_string(),
                        headers,
                        method,
                        path,
                        query,
                    });

                    svc.call(req).and_then(Ok)
                }),
        )
    }
}

fn extract_headers(headers: &HeaderMap) -> HashMap<String, String> {
    headers
        .iter()
        .map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or("").to_string()))
        .collect::<HashMap<_, _>>()
}

fn extract_query(query: &str) -> HashMap<String, String> {
    match serde_urlencoded::from_str::<HashMap<String, String>>(query) {
        Ok(tuples) => tuples,
        Err(_why) => HashMap::new(),
    }
}

//TODO: https://docs.rs/actix-web/1.0.2/actix_web/test/index.html
/*
#[test]
#[cfg(not(target_os = "windows"))] // carllerche/mio#776
fn test_middleware() {
    let (tx, rx) = crate::channel::unbounded();

    let mut srv = ::actix_web::test::TestServer::new(move |app| {
        app.middleware(ShareRequest { tx: tx.clone() })
            .handler(|_| ::actix_web::HttpResponse::Ok())
    });

    let request = srv.get().finish().unwrap();
    let response = srv.execute(request.send()).unwrap();

    assert!(response.status().is_success());
    assert_eq!(rx.len(), 1);

    let request: Result<Request, crate::channel::RecvError> = rx.recv();

    assert!(request.is_ok());
}
*/
