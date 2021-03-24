use crate::{channel::Sender, helper::load_body};
use actix_web::{
    dev::{Payload, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage,
};
use futures::{
    future::{ok, Future, Ready},
    stream,
};
use http::{header::HeaderValue, request::Request};
use std::{
    cell::RefCell,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll},
};

impl<S, B> Transform<S, ServiceRequest> for Sender
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = ShareRequestMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(ShareRequestMiddleware {
            service: Rc::new(RefCell::new(service)),
            tx: self.tx.clone(),
        })
    }
}

pub struct ShareRequestMiddleware<S> {
    service: Rc<RefCell<S>>,
    tx: crossbeam_channel::Sender<Request<Vec<u8>>>,
}

impl<S, B> Service<ServiceRequest> for ShareRequestMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;

    #[allow(clippy::all)]
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, mut req: ServiceRequest) -> Self::Future {
        let svc = self.service.clone();
        let tx = self.tx.clone();

        Box::pin(async move {
            let body = load_body(req.take_payload()).await?.freeze();

            let mut builder = Request::builder();
            {
                if let Some(headers) = builder.headers_mut() {
                    for (key, value) in req.headers().iter() {
                        headers.insert(key, HeaderValue::from(value));
                    }
                }
            }

            let request = builder
                .method(req.method())
                .uri(req.uri())
                .body(body.to_vec())
                .expect("Failed to build request copy!");

            tx.send(request).expect("Failed to send request!");

            req.set_payload(Payload::Stream(Box::pin(stream::once(ok(body)))));

            Ok(svc.call(req).await?)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{
        rt as actix_rt,
        test::{call_service, init_service, read_body, TestRequest},
        web::{route, Payload},
        App, HttpResponse,
    };

    #[actix_rt::test]
    async fn test_middleware() -> Result<(), Error> {
        let (tx, rx) = crossbeam_channel::unbounded();

        let app =
            init_service(App::new().wrap(Sender::new(tx)).default_service(
                route().to(|payload: Payload| HttpResponse::Ok().streaming(payload)),
            ))
            .await;

        let payload = "hello world";

        let req = TestRequest::default()
            .insert_header(("content-type", "text/plain"))
            .set_payload(payload)
            .to_request();
        let res = call_service(&app, req).await;

        assert_eq!(read_body(res).await, payload);

        assert_eq!(rx.len(), 1);

        let recv_req = rx.recv().unwrap();
        assert_eq!(&recv_req.body()[..], payload.as_bytes());
        assert_eq!(
            recv_req.headers().get("content-type").unwrap(),
            "text/plain"
        );

        Ok(())
    }
}
