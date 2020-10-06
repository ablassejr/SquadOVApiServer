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
                .route("/forgotpw", web::post().to(auth::forgot_pw_handler))
        )
}