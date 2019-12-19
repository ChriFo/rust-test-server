use crate::helper::load_body;
use crate::requests::{Request, ShareRequest};
use actix_http::{httpmessage::HttpMessage, Payload};
use actix_service::{Service, Transform};
use actix_web::{
    dev::{ServiceRequest, ServiceResponse},
    Error,
};
use futures::{
    future::{ok, FutureExt, LocalBoxFuture, Ready},
    stream,
};
use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
    task::{Context, Poll},
};

impl<S: 'static, B> Transform<S> for ShareRequest
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = ShareRequestMiddleware<S>;
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
    tx: crossbeam::channel::Sender<Request>,
}

impl<S, B> Service for ShareRequestMiddleware<S>
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, mut req: ServiceRequest) -> Self::Future {
        let mut svc = self.service.clone();
        let tx = self.tx.clone();

        let query = extract_query(req.query_string());

        let method = req.method().to_string();
        let path = req.path().to_string();

        async move {
            let body = load_body(req.take_payload()).await?.freeze();
            let _ = tx.send(Request {
                body: String::from_utf8_lossy(&body.to_vec()).to_string(),
                headers: req.headers().clone(),
                method,
                path,
                query,
            });

            req.set_payload(Payload::Stream(Box::pin(stream::once(ok(body)))));

            Ok(svc.call(req).await?)
        }
            .boxed_local()
    }
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

    #[actix_rt::test]
    async fn test_middleware() -> Result<(), Error> {
        let (tx, rx) = crossbeam::channel::unbounded();

        let mut app =
            init_service(App::new().wrap(ShareRequest { tx }).default_service(
                route().to(|payload: Payload| HttpResponse::Ok().streaming(payload)),
            ))
            .await;

        let payload = "hello world";

        let req = TestRequest::default().set_payload(payload).to_request();
        let res = call_service(&mut app, req).await;

        assert_eq!(read_body(res).await, payload);

        assert_eq!(rx.len(), 1);
        assert_eq!(rx.recv().unwrap().body, payload);

        Ok(())
    }
}
