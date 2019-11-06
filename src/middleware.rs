use crate::channel::Sender;
use crate::requests::{Request, ShareRequest};
use actix_http::{error::PayloadError, httpmessage::HttpMessage, Payload};
use actix_service::{Service, Transform};
use actix_web::{
    dev::{ServiceRequest, ServiceResponse},
    http::header::HeaderMap,
    Error,
};
use futures::{
    future::{ok, FutureResult},
    stream, Future, Poll, Stream,
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

                    req.set_payload(Payload::Stream(Box::new(stream::once(Ok(body)))));

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
        Err(why) => {
            error!("Failed to extract Query");
            debug!("{}", why);
            HashMap::new()
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use actix_web::{
        test::{call_service, init_service, read_body, TestRequest},
        web::{route, Payload},
        App, HttpResponse,
    };

    #[test]
    fn test_middleware() -> Result<(), Error> {
        let (tx, rx) = crate::channel::unbounded();

        let mut app =
            init_service(App::new().wrap(ShareRequest { tx }).default_service(
                route().to(|payload: Payload| HttpResponse::Ok().streaming(payload)),
            ));

        let payload = "hello world";

        let req = TestRequest::default().set_payload(payload).to_request();
        let res = call_service(&mut app, req);

        assert_eq!(read_body(res), payload);

        assert_eq!(rx.len(), 1);
        assert_eq!(rx.recv().unwrap().body, payload);

        Ok(())
    }
}
