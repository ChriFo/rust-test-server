# (Integration) Test server

![GitHub tag](https://img.shields.io/github/tag/ChriFo/test-server-rs.svg)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)

## Usage

```toml
[dev-dependencies]
test-server = { git = "https://github.com/ChriFo/test-server-rs", tag = "v0.2.3" }
```

[HttpResponse](https://actix.rs/api/actix-web/stable/actix_web/struct.HttpResponse.html) and [HttpRequest](https://actix.rs/api/actix-web/stable/actix_web/struct.HttpRequest.html) are re-exports from [actix-web](https://github.com/actix/actix-web).

```rust
extern crate test_server;

use test_server::{HttpResponse, TestServer};

#[test]
fn example_test() {
    // start server at random port
    let _ = TestServer::new(0, |_| HttpResponse::Ok().into());

    // start server at given port
    let server = TestServer::new(8080, |req| {
        println!("Request: {:#?}", req);
        HttpResponse::Ok().body("hello world")
    });

    // request against server
    let _ = client::get(&server.url());

    // get received request as vector from server
    let last_request = server.requests()[0];

    assert_eq!("GET", last_request.method);
    assert_eq!("/", last_request.path);
    // body and headers are also available
}
```

For more examples have a look at the [tests](https://github.com/ChriFo/test-server-rs/blob/master/tests/server.rs).
