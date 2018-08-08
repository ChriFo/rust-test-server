extern crate reqwest;
extern crate test_server;

use reqwest::StatusCode;
use test_server::{helper, HttpResponse, Request, TestServer};

#[test]
fn start_server_at_given_port() {
    let server = TestServer::new(65432, |_| HttpResponse::Ok().into());

    assert!(&server.url().contains(":65432"));

    let response = reqwest::get(&server.url()).unwrap();

    assert_eq!(StatusCode::Ok, response.status());
}

#[test]
#[cfg(not(target_os = "windows"))] // carllerche/mio#776
fn restart_server_at_same_port() {
    let mut server = TestServer::new(65433, |_| HttpResponse::Ok().into());
    let response = reqwest::get(&server.url()).unwrap();

    assert_eq!(StatusCode::Ok, response.status());

    server.stop();
    server = TestServer::new(65433, |_| HttpResponse::BadRequest().into());
    let response = reqwest::get(&server.url()).unwrap();

    assert_eq!(StatusCode::BadRequest, response.status());
}

#[test]
fn validate_client_request() {
    let server = TestServer::new(0, |_| HttpResponse::Ok().into());

    let request_content = helper::create_rand_string(100);
    let client = reqwest::Client::new();
    let _ = client
        .post(&server.url())
        .body(request_content.clone())
        .send();

    let requests = server.requests();

    assert_eq!(requests.len(), 1);

    let Request {
        ref body,
        ref headers,
        ref method,
        ref path,
    } = requests[0];

    assert_eq!(&request_content, body);
    assert_eq!(Some(&String::from("100")), headers.get("content-length"));
    assert_eq!("POST", method);
    assert_eq!("/", path);
}

#[test]
fn not_necessary_to_fetch_request_from_server() {
    let server = TestServer::new(0, |_| {
        let content = helper::read_file("tests/sample.json");
        HttpResponse::Ok().body(content).into()
    });
    let mut response = reqwest::get(&server.url()).unwrap();

    assert_eq!(
        helper::read_file("tests/sample.json"),
        response.text().unwrap()
    );
}

#[test]
fn fetch_2nd_request_from_server() {
    let server = TestServer::new(0, |_| HttpResponse::Ok().into());

    let _ = reqwest::get(&server.url()).unwrap();
    let _ = reqwest::Client::new().post(&server.url()).body("2").send();

    let requests = server.requests();

    assert_eq!(requests.len(), 2);
    assert_eq!("2", requests[1].body);
}
