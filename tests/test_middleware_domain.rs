use poem::{
    get, handler, test::TestClient, Endpoint, EndpointExt, Middleware, Request, Result, Route,
};

use poem_casbin_auth::{CasbinService, CasbinVals};

use casbin::{DefaultModel, FileAdapter};
use poem::http::StatusCode;

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
            domain: Option::from(String::from("domain1")),
        };
        req.extensions_mut().insert(vals);
        self.ep.call(req).await
    }
}

#[handler]
fn endpoint() -> () {}

#[cfg_attr(feature = "runtime-tokio", tokio::test)]
#[cfg_attr(feature = "runtime-async-std", async_std::test)]
async fn test_middleware() {
    let m = DefaultModel::from_file("examples/rbac_with_domains_model.conf")
        .await
        .unwrap();
    let a = FileAdapter::new("examples/rbac_with_domains_policy.csv");

    let casbin_middleware = CasbinService::new(m, a).await.unwrap();

    let app = Route::new()
        .at("/pen/1", get(endpoint))
        .at("/book/1", get(endpoint))
        .with(casbin_middleware)
        .with(FakeAuth);
    let cli = TestClient::new(app);

    let resp_pen = cli.get("/pen/1").send().await;
    resp_pen.assert_status_is_ok();

    let resp_book = cli.get("/book/1").send().await;
    resp_book.assert_status(StatusCode::FORBIDDEN);
}
