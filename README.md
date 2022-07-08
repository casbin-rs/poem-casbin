# Poem Casbin Middleware

[Casbin](https://github.com/casbin/casbin-rs) access control middleware for [poem](https://github.com/poem-web/poem) framework

## Install

Add it to `Cargo.toml`

```toml
poem = "1.3.31"
poem-casbin-auth = "0.x.x"
tokio = { version = "1.17.0", features = ["rt-multi-thread", "macros"] }
```

## Requirement

**Casbin only takes charge of permission control**, so you need to implement an `Authentication Middleware` to identify user.

You should put `poem_casbin_auth::CasbinVals` which contains `subject`(username) and `domain`(optional) into [Extension](https://docs.rs/http/0.2.8/http/struct.Extensions.html).

For example:
```rust
use poem::{
    Endpoint, EndpointExt, Middleware, Request, Result,
};
use poem_casbin_auth::CasbinVals;

pub struct FakeAuth;

pub struct FakeAuthMiddleware<E> {
    ep: E,
}

impl<E: Endpoint> Middleware<E> for FakeAuth {
    type Output = FakeAuthMiddleware<E>;

    fn transform(&self, ep: E) -> Self::Output {
        FakeAuthMiddleware { ep }
    }
}

#[poem::async_trait]
impl<E: Endpoint> Endpoint for FakeAuthMiddleware<E> {
    type Output = E::Output;

    async fn call(&self, mut req: Request) -> Result<Self::Output> {
        let vals = CasbinVals {
            subject: String::from("alice"),
            domain: None,
        };
        req.extensions_mut().insert(vals);
        self.ep.call(req).await
    }
}
```

## Example
```rust
use poem_casbin_auth::casbin::{DefaultModel, FileAdapter, Result};
use poem_casbin_auth::CasbinService;
use poem::{get, handler, listener::TcpListener, web::Path, Route, Server};
use poem_casbin_auth::casbin::function_map::key_match2;

#[allow(dead_code)]
mod fake_auth;

#[handler]
fn endpoint() -> () {}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let m = DefaultModel::from_file("examples/rbac_with_pattern_model.conf")
        .await
        .unwrap();
    let a = FileAdapter::new("examples/rbac_with_pattern_policy.csv");

    let casbin_middleware = CasbinService::new(m, a).await.unwrap();

    casbin_middleware
        .write()
        .await
        .get_role_manager()
        .write()
        .matching_fn(Some(key_match2), None);

    let app = Route::new()
        .at("/pen/1", get(endpoint))
        .at("/pen/2", get(endpoint))
        .at("/book/:id", get(endpoint))
        .with(casbin_middleware)
        .with(FakeAuth);

    Server::new(TcpListener::bind("127.0.0.1:8080"))
        .run(app)
        .await
}
```

## License

This project is licensed under

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or [http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0))
