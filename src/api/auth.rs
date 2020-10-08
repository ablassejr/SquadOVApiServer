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

use std::pin::Pin;
use std::task::{Context, Poll};
use std::rc::Rc;
use std::cell::RefCell;
use actix_service::{Service, Transform};
use actix_web::{dev::ServiceRequest, dev::ServiceResponse, Error, web};
use actix_web::http::{HeaderName, HeaderValue};
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

const SET_SESSION_ID_HEADER_KEY : &str = "x-squadov-set-session-id";

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
            let app = match request.app_data::<web::Data<crate::api::ApiApplication>>() {
                Some(x) => x,
                None => return Err(actix_web::error::ErrorInternalServerError("Bad App Data")),
            };
    
            let session = match app.refresh_and_obtain_valid_session_from_request(&request).await {
                Ok(x) => match x{
                    Some(y) => y,
                    None => return Err(actix_web::error::ErrorUnauthorized("Invalid session.")),
                },
                Err(_) => return Err(actix_web::error::ErrorInternalServerError("Internal error.")),
            };

            // Need to clone the session ID so we can relay it back to the user later if needed.
            let new_session_id = session.session_id.clone();
            let need_session_response = session.old_session_id.is_some();

            {
                let mut extensions = request.extensions_mut();
                extensions.insert(session);
            }

            let mut response = match ServiceRequest::from_parts(request, payload) {
                Ok(x) => srv.call(x).await?,
                Err(_) => return Err(actix_web::error::ErrorInternalServerError("Failed to reconstruct service request"))
            };
    
            if need_session_response {
                let headers = response.headers_mut();
                headers.insert(
                    HeaderName::from_static(SET_SESSION_ID_HEADER_KEY),
                    match HeaderValue::from_str(&new_session_id) {
                        Ok(v) => v,
                        Err(_) => return Err(actix_web::error::ErrorInternalServerError("Failed to set change session header"))
                    },
                );
            }

            return Ok(response);
        })
    }
}