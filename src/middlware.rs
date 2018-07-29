use super::{Request, SendRequest, MAP};
use actix_web::{
    middleware::{Middleware, Started}, Error, HttpMessage, HttpRequest, Result,
};
use bytes::BytesMut;
use futures::{Future, Stream};
use rand::prelude::random;
use std::collections::HashMap;

impl<S: 'static> Middleware<S> for SendRequest {
    fn start(&self, req: &HttpRequest<S>) -> Result<Started> {
        let id: u8 = random();
        self.tx.send(id);

        let headers = req.headers()
            .iter()
            .map(|(k, v)| {
                (
                    k.as_str().to_string(),
                    v.to_str()
                        .expect("Failed to convert header value")
                        .to_string(),
                )
            })
            .collect::<HashMap<_, _>>();

        let method = req.method().to_string();
        let path = req.path().to_string();

        let fut = req.clone()
            .payload()
            .from_err()
            .fold(
                BytesMut::new(),
                move |mut body, chunk| -> Result<_, Error> {
                    body.extend_from_slice(&chunk);
                    Ok(body)
                },
            )
            .and_then(move |body| {
                MAP.lock().insert(
                    id,
                    Request {
                        body: String::from_utf8(body.to_vec())
                            .expect("Failed to extract request body"),
                        headers,
                        method,
                        path,
                    },
                );
                Ok(None)
            });

        Ok(Started::Future(Box::new(fut)))
    }
}
