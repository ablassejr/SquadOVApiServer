mod user_specific_access;

pub use user_specific_access::*;

use squadov_common;
use actix_web::{HttpRequest};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::rc::Rc;
use std::cell::RefCell;
use actix_service::{Service, Transform};
use actix_web::{dev::ServiceRequest, dev::ServiceResponse, Error};
use futures::future::{ok, Ready};
use futures::Future;
use super::auth::SquadOVSession;

/// This trait is used by the access middleware to check to see
/// whether the current user has access to whatever the checker is
/// protecting.
pub trait AccessChecker {
    /// Checks whether or not the current request should have
    /// access to whatever path is being requested. This needs to
    /// be an instance method instead of a static method so that the
    /// instance can be made by the user to hold parameters specific
    /// to that path (e.g. checking whether the user has access to some
    /// resource specifically).
    fn check(&self, session: &SquadOVSession, req: &HttpRequest) -> Result<bool, squadov_common::SquadOvError>;
}

pub struct ApiAccess<T : AccessChecker> {
    pub checker: Rc<RefCell<T>>
}

impl<S, B, T> Transform<S> for ApiAccess<T>
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
    T: AccessChecker + 'static,
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
        })
    }
}

pub struct ApiAccessMiddleware<S, T : AccessChecker> {
    service: Rc<RefCell<S>>,
    checker: Rc<RefCell<T>>
}

impl<S, B, T> Service for ApiAccessMiddleware<S, T>
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
    T: AccessChecker + 'static,
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
        let checker = self.checker.clone();

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

                match checker.borrow().check(session, &request) {
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