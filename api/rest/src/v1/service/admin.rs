use actix_web::{http::StatusCode, web, HttpResponse};
use hb_dao::{admin::AdminDao, Db};
use hb_token_jwt::kind::JwtTokenKind;

use crate::{
    context::ApiRestContext as Context,
    v1::model::{
        admin::{AdminResJson, DeleteAdminResJson, UpdateOneAdminReqJson},
        Response, TokenReqHeader,
    },
};

pub fn admin_api(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/admin")
            .route("", web::get().to(find_one))
            .route("", web::patch().to(update_one))
            .route("", web::delete().to(delete_one)),
    );
}

async fn find_one(ctx: web::Data<Context>, token: web::Header<TokenReqHeader>) -> HttpResponse {
    let token = match token.get() {
        Some(token) => token,
        None => return Response::error(StatusCode::BAD_REQUEST, "Invalid token"),
    };

    let token_claim = match ctx.token.jwt.decode(token) {
        Ok(token) => token,
        Err(err) => return Response::error(StatusCode::BAD_REQUEST, err.to_string().as_str()),
    };

    if token_claim.kind() != &JwtTokenKind::Admin {
        return Response::error(StatusCode::BAD_REQUEST, "Must be logged in as admin");
    }

    let db = Db::ScyllaDb(&ctx.db.scylladb);

    let admin_data = match AdminDao::select(&db, token_claim.id()).await {
        Ok(data) => data,
        Err(err) => {
            return Response::error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string().as_str())
        }
    };

    Response::data(
        StatusCode::OK,
        None,
        AdminResJson::new(
            admin_data.id(),
            admin_data.created_at(),
            admin_data.updated_at(),
            admin_data.email(),
        ),
    )
}

async fn update_one(
    ctx: web::Data<Context>,
    token: web::Header<TokenReqHeader>,
    data: web::Json<UpdateOneAdminReqJson>,
) -> HttpResponse {
    let token = match token.get() {
        Some(token) => token,
        None => return Response::error(StatusCode::BAD_REQUEST, "Invalid token"),
    };

    let token_claim = match ctx.token.jwt.decode(token) {
        Ok(token) => token,
        Err(err) => return Response::error(StatusCode::BAD_REQUEST, err.to_string().as_str()),
    };

    if token_claim.kind() != &JwtTokenKind::Admin {
        return Response::error(StatusCode::BAD_REQUEST, "Must be logged in as admin");
    }

    let db = Db::ScyllaDb(&ctx.db.scylladb);

    let mut admin_data = match AdminDao::select(&db, token_claim.id()).await {
        Ok(data) => data,
        Err(err) => return Response::error(StatusCode::BAD_REQUEST, err.to_string().as_str()),
    };

    // if let Some(email) = data.email() {
    //     admin_data.set_email(email);
    // }

    if let Some(password) = data.password() {
        let password_hash = match ctx.hash.argon2.hash_password(password.as_bytes()) {
            Ok(hash) => hash,
            Err(err) => {
                return Response::error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string().as_str())
            }
        };

        admin_data.set_password_hash(&password_hash.to_string());
    }

    // if data.email().is_some() || data.password().is_some() {
    if data.password().is_some() {
        if let Err(err) = admin_data.update(&db).await {
            return Response::error(StatusCode::BAD_REQUEST, err.to_string().as_str());
        }
    }

    Response::data(
        StatusCode::OK,
        None,
        AdminResJson::new(
            admin_data.id(),
            admin_data.created_at(),
            admin_data.updated_at(),
            admin_data.email(),
        ),
    )
}

async fn delete_one(ctx: web::Data<Context>, token: web::Header<TokenReqHeader>) -> HttpResponse {
    let token = match token.get() {
        Some(token) => token,
        None => return Response::error(StatusCode::BAD_REQUEST, "Invalid token"),
    };

    let token_claim = match ctx.token.jwt.decode(token) {
        Ok(token) => token,
        Err(err) => return Response::error(StatusCode::BAD_REQUEST, err.to_string().as_str()),
    };

    if token_claim.kind() != &JwtTokenKind::Admin {
        return Response::error(StatusCode::BAD_REQUEST, "Must be logged in as admin");
    }

    let db = Db::ScyllaDb(&ctx.db.scylladb);

    if let Err(err) = AdminDao::delete(&db, token_claim.id()).await {
        return Response::error(StatusCode::BAD_REQUEST, err.to_string().as_str());
    }

    Response::data(
        StatusCode::OK,
        None,
        DeleteAdminResJson::new(token_claim.id()),
    )
}
