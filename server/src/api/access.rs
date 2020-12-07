mod user_specific;
mod squad;
mod squad_invite;
mod riot;
mod vod;

pub use user_specific::*;
pub use squad::*;
pub use squad_invite::*;
pub use riot::*;
pub use vod::*;

use squadov_common;
use actix_web::{web, HttpRequest};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::rc::Rc;
use std::cell::RefCell;
use actix_service::{Service, Transform};
use actix_web::{dev::ServiceRequest, dev::ServiceResponse, Error};
use futures::future::{ok, Ready};
use futures::Future;
use super::auth::SquadOVSession;
use crate::api::ApiApplication;
use std::sync::Arc;
use std::boxed::Box;
use async_trait::async_trait;
use std::collections::HashMap;

type TChecker<T> = Rc<RefCell<Box<dyn AccessChecker<T>>>>;

/// This trait is used by the access middleware to check to see
/// whether the current user has access to whatever the checker is
/// protecting.
#[async_trait]
pub trait AccessChecker<T: Send + Sync> {
    /// Checks whether or not the current request should have
    /// access to whatever path is being requested. This needs to
    /// be an instance method instead of a static method so that the
    /// instance can be made by the user to hold parameters specific
    /// to that path (e.g. checking whether the user has access to some
    /// resource specifically).
    async fn check(&self, app: Arc<ApiApplication>, session: &SquadOVSession, data: T) -> Result<bool, squadov_common::SquadOvError>;
    fn generate_aux_metadata(&self, req: &HttpRequest) -> Result<T, squadov_common::SquadOvError>;
}

pub struct ApiAccess<T : Send + Sync> {
    // Default checker when no other matches exist.
    pub checker: TChecker<T>,
    // Checker to use for specific HTTP verbs
    pub verb_checkers: HashMap<String, TChecker<T>>
}

impl<T: Send + Sync> ApiAccess<T> {
    pub fn new(input: Box<dyn AccessChecker<T>>) -> Self {
        Self {
            checker: Rc::new(RefCell::new(input)),
            verb_checkers: HashMap::new()
        }
    }

    pub fn verb_override(mut self, v: &str, c: Box<dyn AccessChecker<T>>) -> Self {
        self.verb_checkers.insert(String::from(v), Rc::new(RefCell::new(c)));
        self
    }
}

impl<S, B, T> Transform<S> for ApiAccess<T>
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
    T: Send + Sync + 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = ApiAccessMiddleware<S, T>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(ApiAccessMiddleware { 
            service: Rc::new(RefCell::new(service)),
            checker: self.checker.clone(),
            verb_checkers: self.verb_checkers.clone(),
        })
    }
}

pub struct ApiAccessMiddleware<S, T : Send + Sync> {
    service: Rc<RefCell<S>>,
    checker: TChecker<T>,
    verb_checkers: HashMap<String, TChecker<T>>
}

impl<S, B, T> Service for ApiAccessMiddleware<S, T>
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
    T: Send + Sync + 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.borrow_mut().poll_ready(cx)
    }

    fn call(&mut self, req: ServiceRequest) -> Self::Future {
        let mut srv = self.service.clone();
        let method = String::from(req.method().as_str());
        let checker = if self.verb_checkers.contains_key(&method) {
            self.verb_checkers.get(&method).unwrap().clone()
        } else {
            self.checker.clone()
        };

        Box::pin(async move {
            let (request, payload) = req.into_parts();

            {
                // We assume that this middleware is used in conjunction with the api::auth::ApiSessionValidatorMiddleware
                // middleware so given that they're logged in, we can obtain their session.
                let extensions = request.extensions();
                let session = match extensions.get::<SquadOVSession>() {
                    Some(x) => x,
                    None => return Err(actix_web::error::ErrorUnauthorized("No session"))
                };

                let app = request.app_data::<web::Data<Arc<ApiApplication>>>();
                if app.is_none() {
                    return Err(actix_web::error::ErrorInternalServerError("No app data."));
                }

                let borrowed_checker = checker.borrow();

                // Obtain aux data from the request necessary for the checker to perform an access check.
                // This is necessary because HttpRequest is not send/sync so we can't pass it to an async call.
                let aux_data = borrowed_checker.generate_aux_metadata(&request)?;
                match checker.borrow().check(app.unwrap().get_ref().clone(), session, aux_data).await {
                    Ok(x) => if x { () } else {  return Err(actix_web::error::ErrorUnauthorized("Access check fail")) },
                    Err(_) => return Err(actix_web::error::ErrorInternalServerError("Failed to perform access check")),
                };
            }

            match ServiceRequest::from_parts(request, payload) {
                Ok(x) => Ok(srv.call(x).await?),
                Err(_) => Err(actix_web::error::ErrorInternalServerError("Failed to reconstruct service request"))
            }
        })
    }
}