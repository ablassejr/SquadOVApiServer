use actix_web::{web};
use actix_web::dev::{HttpServiceFactory};
use super::auth;

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
                .route("/verify", web::get().to(auth::check_verify_email_handler))
                .route("/verify/resend", web::post().to(auth::resend_verify_email_handler))
        )
}