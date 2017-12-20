# An untested test server 

99,9 % of the code is stolen from the hyper [server tests](https://github.com/hyperium/hyper/blob/master/tests/server.rs)!

## Usage

```toml
[dependencies]
test-server = { git = "https://github.com/ChriFo/test-server-rs" }
```

```rust
extern crate test_server;

use test_server::futures;
use test_server::hyper;

#[test]
fn test_with_server() {
    // create a server with ip:port
    let server = test_server::serve(Some(String::from("127.0.0.1:65432")));
    // or random port at 127.0.0.1
    let server = test_server::serve(None);
    let addr = server.addr();

    // set response behavior
    server.reply()
        .status(hyper::Ok)
        .body("0");
    
    // pseudo client to interact with server
    MyClient::do_something();

    // the acctual request from the client
    let (method, uri, version, headers, body) = server.request().unwrap().deconstruct();

    // checks
    assert_eq!(hyper::Method::Post, method);
    assert!(body.concat2().wait().unwrap().is_empty());
```