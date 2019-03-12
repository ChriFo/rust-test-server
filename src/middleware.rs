use crate::requests::{Request, ShareRequest};
use actix_web::{
    middleware::{Middleware, Started},
    Error, HttpMessage, HttpRequest, Result,
};
use bytes::BytesMut;
use futures::{Future, Stream};
use std::collections::HashMap;

impl<S> Middleware<S> for ShareRequest {
    fn start(&self, req: &HttpRequest<S>) -> Result<Started> {
        let tx = self.tx.clone();

        let headers = extract_headers(req);
        let query = extract_query(req);

        let method = req.method().to_string();
        let path = req.path().to_string();

        let fut = req
            .payload()
            .from_err()
            .fold(BytesMut::new(), |mut body, chunk| -> Result<_, Error> {
                body.extend_from_slice(&chunk);
                Ok(body)
            })
            .and_then(move |body| {
                let _ = tx.send(Request {
                    body: String::from_utf8(body.to_vec()).expect("Failed to extract request body"),
                    headers,
                    method,
                    path,
                    query,
                });
                Ok(None)
            });

        Ok(Started::Future(Box::new(fut)))
    }
}

fn extract_headers<S>(req: &HttpRequest<S>) -> HashMap<String, String> {
    req.headers()
        .iter()
        .map(|(k, v)| {
            (
                k.as_str().to_string(),
                v.to_str()
                    .expect("Failed to convert header value")
                    .to_string(),
            )
        })
        .collect::<HashMap<_, _>>()
}

fn extract_query<S>(req: &HttpRequest<S>) -> HashMap<String, String> {
    req.query()
        .iter()
        .map(|(k, v)| (k.as_str().to_string(), v.clone()))
        .collect::<HashMap<_, _>>()
}

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
