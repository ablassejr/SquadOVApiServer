use actix_web::{web};
use actix_web::dev::{HttpServiceFactory};
use super::auth;
use super::v1;
use super::access;
use std::rc::Rc;
use std::cell::RefCell;

pub fn create_service() -> impl HttpServiceFactory {
    return web::scope("")
        .service(
            web::scope("/auth")
                .route("/login", web::post().to(auth::login_handler))
                .route("/logout", web::post().to(auth::logout_handler))
                .route("/register", web::post().to(auth::register_handler))
                .route("/forgotpw", web::get().to(auth::forgot_pw_handler))
                .route("/forgotpw/change", web::post().to(auth::forgot_pw_change_handler))
                .route("/verify", web::post().to(auth::verify_email_handler))
                .service(
                    // These are the only two endpoints where the user needs to provide a valid session to use.
                    web::scope("")
                        .wrap(auth::ApiSessionValidator{})
                        .route("/verify", web::get().to(auth::check_verify_email_handler))
                        .route("/verify/resend", web::post().to(auth::resend_verify_email_handler))
                )
        )
        .service(
            web::scope("/v1")
                .wrap(auth::ApiSessionValidator{})
                .service(
                    web::scope("/users")
                        .service(
                            web::scope("/{user_id}")
                                .wrap(access::ApiAccess{
                                    checker: Rc::new(RefCell::new(access::UserSpecificAccessChecker{
                                        obtainer: access::UserIdPathSetObtainer{
                                            key: "user_id"
                                        },
                                    })),
                                })
                                .service(
                                    web::resource("/profile")
                                        .route(web::get().to(v1::get_user_profile_handler))
                                )
                        )
                )
        )
}