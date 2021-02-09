mod user;
mod login;
mod register;
mod forgot_password;
mod verify_email;
mod logout;
mod session;

pub use user::*;
pub use login::*;
pub use register::*;
pub use forgot_password::*;
pub use verify_email::*;
pub use logout::*;
pub use session::*;

use squadov_common::SquadOvError;
use std::sync::Arc;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::rc::Rc;
use std::cell::RefCell;
use actix_service::{Service, Transform};
use actix_web::{dev::ServiceRequest, dev::ServiceResponse, Error, web};
use futures::future::{ok, Ready};
use futures::Future;

/// Session validation middleware.
/// 
/// This middleware will automatically process the request, determine the
/// session, and refresh the session access token if necessary. This middleware
/// will abort the request with an unauthorized response if no session is found
/// or if an invalid session is passed in.
pub struct ApiSessionValidator;

impl<S, B> Transform<S> for ApiSessionValidator
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = ApiSessionValidatorMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(ApiSessionValidatorMiddleware { 
            service: Rc::new(RefCell::new(service)),
        })
    }
}

pub struct ApiSessionValidatorMiddleware<S> {
    service: Rc<RefCell<S>>,
}

impl<S, B> Service for ApiSessionValidatorMiddleware<S>
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
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

        Box::pin(async move {
            let (request, payload) = req.into_parts();

            // The docs say to use app::data::<web::Data<T>> when we store the data using
            // App::data...but we're using App::app_data but if we don't retrieve a web::Data the
            // Option return none. /shrug.
            let app = match request.app_data::<web::Data<Arc<crate::api::ApiApplication>>>() {
                Some(x) => x,
                None => return Err(actix_web::error::ErrorInternalServerError("Bad App Data")),
            };

            // First check for the presence of a valid share key. This takes precendence over everything else.
            let share_token = app.session.get_share_token_from_request(&request, &app.config.squadov.share_key).await?;
            let session = if share_token.is_some() {
                let share_token = share_token.unwrap();
                SquadOVSession{
                    session_id: String::new(),
                    user: app.users.get_stored_user_from_uuid(&share_token.user_uuid, &*app.pool).await?.ok_or(SquadOvError::NotFound)?,
                    access_token: String::new(),
                    refresh_token: String::new(),
                    old_session_id: None,
                    is_temp: true,
                    share_token: Some(share_token),
                }
            } else {
                match app.session.get_session_from_request(&request, &app.pool).await {
                    Ok(x) => match x {
                        Some(s) => s,
                        None => return Err(actix_web::error::ErrorUnauthorized("No Session")),
                    },
                    Err(_) => return Err(actix_web::error::ErrorInternalServerError("Could not retrieve session")),
                }
            };
            
            match app.is_session_valid(&session).await {
                Ok(b) => {
                    if !b {
                        return Err(actix_web::error::ErrorUnauthorized("Invalid session"))
                    }
                }
                Err(_) => return Err(actix_web::error::ErrorInternalServerError("Could not check valid session")),
            };

            {
                let mut extensions = request.extensions_mut();
                extensions.insert(session);
            }

            let response = match ServiceRequest::from_parts(request, payload) {
                Ok(x) => srv.call(x).await?,
                Err(_) => return Err(actix_web::error::ErrorInternalServerError("Failed to reconstruct service request"))
            };

            return Ok(response);
        })
    }
}

static INTERNAL_API_KEY: &'static str = "9e9109d2e2772d6fe1408878c009708ddd4f55f7c68346f61089740b0ca9c35f";
/// Internal API key validator.
///
/// We need a way of protecting the internal API as a fallback for
/// when/if the network based (NGINX) rules fail. So for now just have
/// a hard-coded API key that we expect.
pub struct InternalApiKeyValidator;

impl<S, B> Transform<S> for InternalApiKeyValidator
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = InternalApiKeyValidatorMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(InternalApiKeyValidatorMiddleware { 
            service: Rc::new(RefCell::new(service)),
        })
    }
}

pub struct InternalApiKeyValidatorMiddleware<S> {
    service: Rc<RefCell<S>>,
}

impl<S, B> Service for InternalApiKeyValidatorMiddleware<S>
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
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

        Box::pin(async move {
            let (request, payload) = req.into_parts();
            let headers = request.headers();

            match headers.get("Authorization") {
                Some(x) => {
                    match x.to_str() {
                        Ok(token) => {
                            if token.trim_start_matches("Bearer ") != INTERNAL_API_KEY {
                                return Err(actix_web::error::ErrorUnauthorized("Invalid bearer token"));
                            }
                        }
                        Err(_) => return Err(actix_web::error::ErrorUnauthorized("Invalid bearer token"))
                    }
                }
                None => return Err(actix_web::error::ErrorUnauthorized("Invalid bearer token"))
            };

            match ServiceRequest::from_parts(request, payload) {
                Ok(x) => srv.call(x).await,
                Err(_) => return Err(actix_web::error::ErrorInternalServerError("Failed to reconstruct service request"))
            }
        })
    }
}