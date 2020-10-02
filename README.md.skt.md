```rust,skt-test
extern crate test_server;
extern crate ureq;

fn get_request(url: &str) -> Result<ureq::Response, ureq::Error> {{
    ureq::get(url).call()
}}

fn main() {{

    {}

}}
```
