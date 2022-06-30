use poem::{
    get, handler, test::TestClient, Endpoint, EndpointExt, Middleware, Request, Result, Route,
};

use poem_casbin_auth::{CasbinService, CasbinVals};

use casbin::function_map::key_match2;
use casbin::{CachedEnforcer, CoreApi, DefaultModel, FileAdapter};
use poem::http::StatusCode;

use std::sync::Arc;

#[cfg(feature = "runtime-tokio")]
use tokio::sync::RwLock;

#[cfg(feature = "runtime-async-std")]
use async_std::sync::RwLock;

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

#[handler]
fn endpoint() -> () {}

#[cfg_attr(feature = "runtime-tokio", tokio::test)]
#[cfg_attr(feature = "runtime-async-std", async_std::test)]
async fn test_set_enforcer() {
    let m = DefaultModel::from_file("examples/rbac_with_pattern_model.conf")
        .await
        .unwrap();
    let a = FileAdapter::new("examples/rbac_with_pattern_policy.csv");

    let enforcer = Arc::new(RwLock::new(CachedEnforcer::new(m, a).await.unwrap()));

    let casbin_middleware = CasbinService::set_enforcer(enforcer);

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
    let cli = TestClient::new(app);

    let resp_pen_1 = cli.get("/pen/1").send().await;
    resp_pen_1.assert_status_is_ok();

    let resp_book = cli.get("/book/2").send().await;
    resp_book.assert_status_is_ok();

    let resp_pen_2 = cli.get("/pen/2").send().await;
    resp_pen_2.assert_status(StatusCode::FORBIDDEN);
}
