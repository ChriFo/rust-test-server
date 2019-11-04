use crate::channel::{Receiver, Sender};
use std::{collections::HashMap, rc::Rc};

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
    pub rx: Rc<Receiver<Request>>,
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
    pub tx: Sender<Request>,
}

impl ShareRequest {
    pub fn new(tx: Sender<Request>) -> Self {
        Self { tx }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crossbeam_channel::Sender;
    use std::rc::Rc;

    #[test]
    fn request_receiver_is_empty() {
        let (tx, rx) = crossbeam_channel::unbounded();
        let rr = RequestReceiver { rx: Rc::new(rx) };

        assert!(rr.is_empty());

        add_request(tx);

        assert!(!rr.is_empty());
    }

    #[test]
    fn request_reciever_len() {
        let (tx, rx) = crossbeam_channel::unbounded();
        let rr = RequestReceiver { rx: Rc::new(rx) };

        assert_eq!(rr.len(), 0);

        add_request(tx);

        assert_eq!(rr.len(), 1);
    }

    #[test]
    fn request_reciever_next() {
        let (tx, rx) = crossbeam_channel::unbounded();
        let rr = RequestReceiver { rx: Rc::new(rx) };

        assert!(rr.next().is_none());

        add_request(tx);

        assert!(rr.next().is_some());
    }

    fn add_request(tx: Sender<Request>) {
        if let Err(err) = tx.send(Request {
            body: String::new(),
            headers: HashMap::new(),
            method: String::new(),
            path: String::new(),
            query: HashMap::new(),
        }) {
            error!("Failed to send Request");
            debug!("{}", err);
        }
    }
}
