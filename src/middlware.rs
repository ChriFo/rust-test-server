use super::{Request, ShareRequest, QUEUE};
use actix_web::{
    middleware::{Middleware, Started},
    Error, HttpMessage, HttpRequest, Result,
};
use bytes::BytesMut;
use futures::{Future, Stream};
use std::collections::HashMap;

impl<S> Middleware<S> for ShareRequest {
    fn start(&self, req: &HttpRequest<S>) -> Result<Started> {
        let id = self.id;

        let headers = req
            .headers()
            .iter()
            .map(|(k, v)| {
                (
                    k.as_str().to_string(),
                    v.to_str()
                        .expect("Failed to convert header value")
                        .to_string(),
                )
            }).collect::<HashMap<_, _>>();

        let method = req.method().to_string();
        let path = req.path().to_string();

        let fut = req
            .payload()
            .from_err()
            .fold(BytesMut::new(), |mut body, chunk| -> Result<_, Error> {
                body.extend_from_slice(&chunk);
                Ok(body)
            }).and_then(move |body| {
                let mut queue = match QUEUE.lock().remove(&id) {
                    Some(queue) => queue,
                    None => vec![],
                };

                queue.push(Request {
                    body: String::from_utf8(body.to_vec()).expect("Failed to extract request body"),
                    headers,
                    method,
                    path,
                });

                QUEUE.lock().insert(id, queue);

                Ok(None)
            });

        Ok(Started::Future(Box::new(fut)))
    }
}
