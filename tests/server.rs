use crate::server::{helper, HttpResponse, Payload, Request};
use failure::Error;
use test_server as server;

#[test]
fn start_server_at_given_port() -> Result<(), Error> {
    let server = server::new(65432, HttpResponse::Ok)?;

    assert!(&server.url().contains(":65432"));

    let response = ureq::get(&server.url()).call();

    assert!(response.ok());

    Ok(())
}

#[test]
#[cfg(not(target_os = "windows"))] // known issue of Windows
fn restart_server_at_same_port() -> Result<(), Error> {
    {
        let server = server::new(65433, HttpResponse::Ok)?;
        let response = ureq::get(&server.url()).call();

        assert!(response.ok());

        server.stop();
    }

    {
        let server = server::new(65433, HttpResponse::BadRequest)?;
        let response = ureq::get(&server.url()).call();

        assert!(response.client_error());
    }

    Ok(())
}

#[test]
fn validate_client_request() -> Result<(), Error> {
    let server = server::new(0, HttpResponse::Ok)?;

    let request_content = helper::random_string(100);
    let _ = ureq::post(&server.url()).send_string(&request_content);

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
    assert_eq!(
        &String::from("100"),
        headers.get("content-length").unwrap().to_str()?
    );
    assert_eq!("POST", method);
    assert_eq!("/", path);
    assert!(query.is_empty());

    Ok(())
}

#[test]
fn validate_client_response() -> Result<(), Error> {
    let server = server::new(0, |payload: Payload| HttpResponse::Ok().streaming(payload))?;

    let request_content = helper::random_string(100);
    let response = ureq::post(&server.url()).send_string(&request_content);

    assert!(response.ok());
    assert_eq!(response.into_string()?, request_content);

    Ok(())
}

#[test]
fn not_necessary_to_fetch_request_from_server() -> Result<(), Error> {
    let server = server::new(0, || {
        let content = helper::read_file("tests/sample.json").unwrap();
        HttpResponse::Ok().body(content)
    })?;
    let response = ureq::get(&server.url()).call();

    assert_eq!(
        helper::read_file("tests/sample.json")?,
        response.into_string()?
    );

    Ok(())
}

#[test]
fn fetch_2nd_request_from_server() -> Result<(), Error> {
    let server = server::new(0, HttpResponse::Ok)?;

    let _ = ureq::get(&server.url()).call();
    let _ = ureq::post(&server.url()).send_string("2");

    assert_eq!(server.requests.len(), 2);

    let _ = server.requests.next();
    let request = server.requests.next();

    assert!(request.is_some());
    assert_eq!("2", request.unwrap().body);

    Ok(())
}
