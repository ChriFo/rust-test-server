use std::collections::HashMap;

#[derive(Debug)]
pub struct Request {
    pub body: String,
    pub headers: HashMap<String, String>,
    pub method: String,
    pub path: String,
    pub query: HashMap<String, String>,
}

#[derive(Debug)]
pub struct RequestReceiver {
    pub rx: crate::channel::Receiver<Request>,
}

impl RequestReceiver {
    pub fn is_empty(&self) -> bool {
        self.rx.len() == 0
    }

    pub fn len(&self) -> usize {
        self.rx.len()
    }

    pub fn next(&self) -> Option<Request> {
        self.rx.try_recv().ok()
    }
}

pub(crate) struct ShareRequest {
    pub(crate) tx: crate::channel::Sender<Request>,
}
