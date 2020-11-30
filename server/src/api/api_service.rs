use actix_web::{web, FromRequest};
use actix_web::dev::{HttpServiceFactory};
use super::auth;
use super::v1;
use super::access;
use super::graphql;
use std::rc::Rc;
use std::cell::RefCell;
use std::vec::Vec;

pub fn create_service(graphql_debug: bool) -> impl HttpServiceFactory {
    let mut scope = web::scope("")
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
                            web::scope("/me")
                                .service(
                                    web::resource("/profile")
                                        .route(web::get().to(v1::get_current_user_profile_handler))
                                )
                        )
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
                .service(
                    web::scope("/valorant")
                        .route("", web::post().to(v1::create_new_valorant_match_handler))
                            .data(web::Json::<v1::InputValorantMatch>::configure(|cfg| {
                                // Bump up the size limit on this endpoint for now because
                                // the user will have to send the entire match detail. Not
                                // sure how large that can be so setting a *really* large
                                // limit here. This should be about 10 MB.
                                cfg.limit(10 * 1024 * 1024)
                            }))
                        .route("/backfill", web::post().to(v1::obtain_valorant_matches_to_backfill))
                        .service(
                            web::scope("/accounts/{puuid}")
                                .service(
                                    web::resource("/matches")
                                        .route(web::get().to(v1::list_valorant_matches_for_user_handler))
                                )
                                .service(
                                    web::resource("/stats")
                                        .route(web::get().to(v1::get_player_stats_summary_handler))
                                )
                        )
                        .service(
                            web::scope("/match/{match_id}")
                                .service(
                                    web::resource("")
                                        .route(web::get().to(v1::get_valorant_match_details_handler))
                                )
                                .service(
                                    web::resource("/metadata/{puuid}")
                                        .route(web::get().to(v1::get_valorant_player_match_metadata_handler))
                                )
                        )
                )
                .service(
                    web::scope("/aimlab")
                        .route("", web::post().to(v1::create_new_aimlab_task_handler))
                        .service(
                            web::scope("/user/{user_id}")
                                .wrap(access::ApiAccess{
                                    checker: Rc::new(RefCell::new(access::UserSpecificAccessChecker{
                                        obtainer: access::UserIdPathSetObtainer{
                                            key: "user_id"
                                        },
                                    })),
                                })
                                .route("", web::get().to(v1::list_aimlab_matches_for_user_handler))
                        )
                        .route("/bulk", web::post().to(v1::bulk_create_aimlab_task_handler))
                            .data(web::Json::<Vec<v1::AimlabTask>>::configure(|cfg| {
                                cfg.limit(1 * 1024 * 1024)
                            }))
                        .service(
                            web::scope("/match/{match_uuid}")
                                .service(
                                    web::resource("/task")
                                        .route(web::get().to(v1::get_aimlab_task_data_handler))
                                )
                        )
                )
                .service(
                    web::scope("/hearthstone")
                        .service(
                            web::scope("/match")
                                .route("", web::post().to(v1::create_hearthstone_match_handler))
                                .service(
                                    web::scope("/{match_uuid}")
                                        .route("", web::post().to(v1::upload_hearthstone_logs_handler))
                                            .data(web::Payload::configure(|cfg| {
                                                // Note that we should be submitting GZIP here so this shouldn't get super super large.
                                                cfg.limit(5 * 1024 * 1024)
                                            }))
                                        .route("", web::get().to(v1::get_hearthstone_match_handler))
                                        .route("/logs", web::get().to(v1::get_hearthstone_match_logs_handler))
                                )
                        )
                        .service(
                            web::scope("/user/{user_id}")
                                .wrap(access::ApiAccess{
                                    checker: Rc::new(RefCell::new(access::UserSpecificAccessChecker{
                                        obtainer: access::UserIdPathSetObtainer{
                                            key: "user_id"
                                        },
                                    })),
                                })
                                .route("/match", web::get().to(v1::list_hearthstone_matches_for_user_handler))
                                .service(
                                    web::scope("/arena")
                                        .route("", web::get().to(v1::list_arena_runs_for_user_handler))
                                        .route("", web::post().to(v1::create_or_retrieve_arena_draft_for_user_handler))
                                        .service(
                                            web::scope("/{collection_uuid}")
                                                .route("", web::post().to(v1::add_hearthstone_card_to_arena_deck_handler))
                                                .route("", web::get().to(v1::get_hearthstone_arena_run_handler))
                                                .route("/matches", web::get().to(v1::list_matches_for_arena_run_handler))
                                                .route("/deck", web::post().to(v1::create_finished_arena_draft_deck_handler))
                                        )
                                )
                                .service(
                                    web::scope("/duels")
                                        .route("", web::get().to(v1::list_duel_runs_for_user_handler))
                                        .service(
                                            web::scope("/{collection_uuid}")
                                                .route("", web::get().to(v1::get_hearthstone_duel_run_handler))
                                                .route("/matches", web::get().to(v1::list_matches_for_duel_run_handler))
                                        )
                                )
                        )
                        .service(
                            web::scope("/cards")
                                .route("", web::post().to(v1::bulk_get_hearthstone_cards_metadata_handler))
                                .route("/battlegrounds/tavern/{tavern_level}", web::get().to(v1::get_battleground_tavern_level_cards_handler))
                        )
                )
                .service(
                    web::scope("/vod")
                        .route("", web::post().to(v1::create_vod_destination_handler))
                        .service(
                            web::scope("/match/{match_uuid}")
                                .service(
                                    web::scope("/user/{user_uuid}")
                                        .service(
                                            web::resource("")
                                                .route(web::get().to(v1::find_vod_from_match_user_handler))
                                        )
                                )
                        )
                        .service(
                            web::scope("/{video_uuid}")
                                .service(
                                    web::resource("")
                                        .route(web::delete().to(v1::delete_vod_handler))
                                        .route(web::get().to(v1::get_vod_handler))
                                        .route(web::post().to(v1::associate_vod_handler))
                                )
                                .service(
                                    web::resource("/{quality}/{segment_name}")
                                        .route(web::get().to(v1::get_vod_track_segment_handler))
                                )
                        )
                )
        );

    if graphql_debug {
        scope = scope.service(
            web::resource("/graphql")
                    .route(web::post().to(graphql::graphql_handler))
                    .route(web::get().to(graphql::graphiql_handler))
        );
    } else {
        scope = scope.service(
            web::resource("/graphql")
                    .wrap(auth::ApiSessionValidator{})
                    .route(web::post().to(graphql::graphql_handler))
        );
    }
    scope
}