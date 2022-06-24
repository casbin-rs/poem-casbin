use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use casbin::prelude::{TryIntoAdapter, TryIntoModel};
use casbin::{CachedEnforcer, CoreApi, Result as CasbinResult};

use poem::{http::StatusCode, Endpoint, Error, Middleware, Request, Result};

#[cfg(feature = "runtime-tokio")]
use tokio::sync::RwLock;

#[cfg(feature = "runtime-async-std")]
use async_std::sync::RwLock;

#[derive(Clone)]
pub struct CasbinVals {
    pub subject: String,
    pub domain: Option<String>,
}

impl CasbinVals {
    pub fn new(subject: String, domain: Option<String>) -> CasbinVals {
        CasbinVals { subject, domain }
    }
}

#[derive(Clone)]
pub struct CasbinService {
    enforcer: Arc<RwLock<CachedEnforcer>>,
}

impl Deref for CasbinService {
    type Target = Arc<RwLock<CachedEnforcer>>;

    fn deref(&self) -> &Self::Target {
        &self.enforcer
    }
}

impl DerefMut for CasbinService {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.enforcer
    }
}

impl CasbinService {
    pub async fn new<M: TryIntoModel, A: TryIntoAdapter>(m: M, a: A) -> CasbinResult<Self> {
        let enforcer: CachedEnforcer = CachedEnforcer::new(m, a).await?;
        Ok(CasbinService {
            enforcer: Arc::new(RwLock::new(enforcer)),
        })
    }

    pub fn get_enforcer(&mut self) -> Arc<RwLock<CachedEnforcer>> {
        self.enforcer.clone()
    }

    pub fn set_enforcer(e: Arc<RwLock<CachedEnforcer>>) -> CasbinService {
        CasbinService { enforcer: e }
    }
}

pub struct CasbinMiddleware<E> {
    ep: E,
    enforcer: Arc<RwLock<CachedEnforcer>>,
}

impl<E: Endpoint> Middleware<E> for CasbinService {
    type Output = CasbinMiddleware<E>;

    fn transform(&self, ep: E) -> Self::Output {
        CasbinMiddleware {
            ep,
            enforcer: self.enforcer.clone(),
        }
    }
}

#[poem::async_trait]
impl<E: Endpoint> Endpoint for CasbinMiddleware<E> {
    type Output = E::Output;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        let path = req.uri().path().to_string();
        let action = req.method().as_str().to_string();
        let option_vals = req.extensions().get::<CasbinVals>().map(|x| x.to_owned());
        let vals = match option_vals {
            Some(value) => value,
            None => {
                return Err(Error::from_status(StatusCode::UNAUTHORIZED));
            }
        };
        let subject = vals.subject.clone();

        if !vals.subject.is_empty() {
            let mut lock = self.enforcer.write().await;
            match if let Some(domain) = vals.domain {
                lock.enforce_mut(vec![subject, domain, path, action])
            } else {
                lock.enforce_mut(vec![subject, path, action])
            } {
                Ok(true) => {
                    drop(lock);
                    self.ep.call(req).await
                }
                Ok(false) => {
                    drop(lock);
                    Err(Error::from_status(StatusCode::FORBIDDEN))
                }
                Err(_) => {
                    drop(lock);
                    Err(Error::from_status(StatusCode::BAD_GATEWAY))
                }
            }
        } else {
            Err(Error::from_status(StatusCode::UNAUTHORIZED))
        }
    }
}
