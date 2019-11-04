use crate::server::{helper, HttpResponse, Payload, Request};
use failure::Error;
use reqwest::{Client, StatusCode};
use test_server as server;

#[test]
fn start_server_at_given_port() -> Result<(), Error> {
    let server = server::new(65432, HttpResponse::Ok)?;

    assert!(&server.url().contains(":65432"));

    let response = reqwest::get(&server.url())?;

    assert_eq!(StatusCode::OK, response.status());

    Ok(())
}

#[test]
fn restart_server_at_same_port() -> Result<(), Error> {
    let mut server = server::new(65433, HttpResponse::Ok)?;
    let response = reqwest::get(&server.url())?;

    assert_eq!(StatusCode::OK, response.status());

    server.stop();
    server = server::new(65433, HttpResponse::BadRequest)?;
    let response = reqwest::get(&server.url())?;

    assert_eq!(StatusCode::BAD_REQUEST, response.status());

    Ok(())
}

#[test]
fn validate_client_request() -> Result<(), Error> {
    let server = server::new(0, HttpResponse::Ok)?;

    let request_content = helper::random_string(100);
    let _ = Client::new()
        .post(&server.url())
        .body(request_content.to_owned())
        .send();

    assert_eq!(server.requests.len(), 1);

    let request = server.requests.next();
    assert!(request.is_some());

    let Request {
        ref body,
        ref headers,
        ref method,
        ref path,
        ref query,
    } = request.unwrap();

    assert_eq!(&request_content, body);
    assert_eq!(Some(&String::from("100")), headers.get("content-length"));
    assert_eq!("POST", method);
    assert_eq!("/", path);
    assert!(query.is_empty());

    Ok(())
}

#[test]
fn validate_client_response() -> Result<(), Error> {
    let server = server::new(0, |payload: Payload| HttpResponse::Ok().streaming(payload))?;

    let request_content = helper::random_string(100);
    let response = Client::new()
        .post(&server.url())
        .body(request_content.to_owned())
        .send();

    assert!(response.is_ok());

    let mut response = response?;
    assert_eq!(response.text()?, request_content);
    assert_eq!(response.status(), StatusCode::OK);

    Ok(())
}

#[test]
fn not_necessary_to_fetch_request_from_server() -> Result<(), Error> {
    let server = server::new(0, || {
        let content = helper::read_file("tests/sample.json").unwrap();
        HttpResponse::Ok().body(content)
    })?;
    let mut response = reqwest::get(&server.url())?;

    assert_eq!(helper::read_file("tests/sample.json")?, response.text()?);

    Ok(())
}

#[test]
fn fetch_2nd_request_from_server() -> Result<(), Error> {
    let server = server::new(0, HttpResponse::Ok)?;

    let _ = reqwest::get(&server.url())?;
    let _ = Client::new().post(&server.url()).body("2").send();

    assert_eq!(server.requests.len(), 2);

    let _ = server.requests.next();
    let request = server.requests.next();

    assert!(request.is_some());
    assert_eq!("2", request.unwrap().body);

    Ok(())
}
