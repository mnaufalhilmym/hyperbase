use actix_web::{http::StatusCode, web, HttpResponse};
use actix_web_httpauth::extractors::bearer::BearerAuth;
use hb_dao::{
    admin::AdminDao,
    bucket::BucketDao,
    bucket_rule::{BucketPermission, BucketRuleDao},
    project::ProjectDao,
    token::TokenDao,
};
use hb_token_jwt::claim::ClaimId;

use crate::{
    context::ApiRestCtx,
    model::{
        bucket_rule::{
            BucketRuleResJson, DeleteBucketRuleResJson, DeleteOneBucketRuleReqPath,
            FindManyBucketRuleReqPath, FindOneBucketRuleReqPath, InsertOneBucketRuleReqJson,
            InsertOneBucketRuleReqPath, UpdateOneBucketRuleReqJson, UpdateOneBucketRuleReqPath,
        },
        PaginationRes, Response,
    },
};

pub fn bucket_rule_api(cfg: &mut web::ServiceConfig) {
    cfg.route(
        "/project/{project_id}/token/{token_id}/bucket_rule",
        web::post().to(insert_one),
    )
    .route(
        "/project/{project_id}/token/{token_id}/bucket_rule/{rule_id}",
        web::get().to(find_one),
    )
    .route(
        "/project/{project_id}/token/{token_id}/bucket_rule/{rule_id}",
        web::patch().to(update_one),
    )
    .route(
        "/project/{project_id}/token/{token_id}/bucket_rule/{rule_id}",
        web::delete().to(delete_one),
    )
    .route(
        "/project/{project_id}/token/{token_id}/bucket_rules",
        web::get().to(find_many),
    );
}

async fn insert_one(
    ctx: web::Data<ApiRestCtx>,
    auth: BearerAuth,
    path: web::Path<InsertOneBucketRuleReqPath>,
    data: web::Json<InsertOneBucketRuleReqJson>,
) -> HttpResponse {
    let token = auth.token();

    let token_claim = match ctx.token().jwt().decode(token) {
        Ok(token) => token,
        Err(err) => return Response::error_raw(&StatusCode::BAD_REQUEST, &err.to_string()),
    };

    let admin_id = match token_claim.id() {
        ClaimId::Admin(id) => match AdminDao::db_select(ctx.dao().db(), id).await {
            Ok(data) => *data.id(),
            Err(err) => {
                return Response::error_raw(
                    &StatusCode::UNAUTHORIZED,
                    &format!("Failed to get admin data: {err}"),
                )
            }
        },
        ClaimId::Token(_, _) => {
            return Response::error_raw(
                &StatusCode::BAD_REQUEST,
                "Must be logged in using password-based login",
            )
        }
    };

    let (project_data, token_data, bucket_data) = match tokio::try_join!(
        ProjectDao::db_select(ctx.dao().db(), path.project_id()),
        TokenDao::db_select(ctx.dao().db(), path.token_id()),
        BucketDao::db_select(ctx.dao().db(), data.bucket_id())
    ) {
        Ok(data) => data,
        Err(err) => return Response::error_raw(&StatusCode::BAD_REQUEST, &err.to_string()),
    };

    if project_data.admin_id() != &admin_id {
        return Response::error_raw(
            &StatusCode::FORBIDDEN,
            "This project does not belong to you",
        );
    }

    if token_data.project_id() != project_data.id() {
        return Response::error_raw(&StatusCode::FORBIDDEN, "This token does not belong to you");
    }

    if bucket_data.project_id() != project_data.id() {
        return Response::error_raw(&StatusCode::FORBIDDEN, "This bucket does not belong to you");
    }

    let rule_find_one = match BucketPermission::from_str(data.find_one()) {
        Ok(rule) => rule,
        Err(err) => return Response::error_raw(&StatusCode::BAD_REQUEST, &err.to_string()),
    };
    let rule_find_many = match BucketPermission::from_str(data.find_many()) {
        Ok(rule) => rule,
        Err(err) => return Response::error_raw(&StatusCode::BAD_REQUEST, &err.to_string()),
    };
    let rule_update_one = match BucketPermission::from_str(data.update_one()) {
        Ok(rule) => rule,
        Err(err) => return Response::error_raw(&StatusCode::BAD_REQUEST, &err.to_string()),
    };
    let rule_delete_one = match BucketPermission::from_str(data.delete_one()) {
        Ok(rule) => rule,
        Err(err) => return Response::error_raw(&StatusCode::BAD_REQUEST, &err.to_string()),
    };

    let bucket_rule_data = BucketRuleDao::new(
        project_data.id(),
        token_data.id(),
        bucket_data.id(),
        &rule_find_one,
        &rule_find_many,
        data.insert_one(),
        &rule_update_one,
        &rule_delete_one,
    );

    if let Err(err) = bucket_rule_data.db_insert(ctx.dao().db()).await {
        return Response::error_raw(&StatusCode::INTERNAL_SERVER_ERROR, &err.to_string());
    }

    Response::data(
        &StatusCode::CREATED,
        &None,
        &BucketRuleResJson::new(
            bucket_rule_data.id(),
            bucket_rule_data.created_at(),
            bucket_rule_data.updated_at(),
            bucket_rule_data.project_id(),
            bucket_rule_data.token_id(),
            bucket_rule_data.bucket_id(),
            bucket_rule_data.find_one().to_str(),
            bucket_rule_data.find_many().to_str(),
            bucket_rule_data.insert_one(),
            bucket_rule_data.update_one().to_str(),
            bucket_rule_data.delete_one().to_str(),
        ),
    )
}

async fn find_one(
    ctx: web::Data<ApiRestCtx>,
    auth: BearerAuth,
    path: web::Path<FindOneBucketRuleReqPath>,
) -> HttpResponse {
    let token = auth.token();

    let token_claim = match ctx.token().jwt().decode(token) {
        Ok(token) => token,
        Err(err) => return Response::error_raw(&StatusCode::BAD_REQUEST, &err.to_string()),
    };

    let admin_id = match token_claim.id() {
        ClaimId::Admin(id) => match AdminDao::db_select(ctx.dao().db(), id).await {
            Ok(data) => *data.id(),
            Err(err) => {
                return Response::error_raw(
                    &StatusCode::UNAUTHORIZED,
                    &format!("Failed to get admin data: {err}"),
                )
            }
        },
        ClaimId::Token(_, _) => {
            return Response::error_raw(
                &StatusCode::BAD_REQUEST,
                "Must be logged in using password-based login",
            )
        }
    };

    let (token_data, bucket_rule_data) = match tokio::try_join!(
        TokenDao::db_select(ctx.dao().db(), path.token_id()),
        BucketRuleDao::db_select(ctx.dao().db(), path.rule_id())
    ) {
        Ok(data) => data,
        Err(err) => return Response::error_raw(&StatusCode::BAD_REQUEST, &err.to_string()),
    };

    if token_data.admin_id() != &admin_id {
        return Response::error_raw(&StatusCode::FORBIDDEN, "This token does not belong to you");
    }

    if bucket_rule_data.token_id() != token_data.id() {
        return Response::error_raw(
            &StatusCode::FORBIDDEN,
            "This bucket rule does not belong to you",
        );
    }

    Response::data(
        &StatusCode::OK,
        &None,
        &BucketRuleResJson::new(
            bucket_rule_data.id(),
            bucket_rule_data.created_at(),
            bucket_rule_data.updated_at(),
            bucket_rule_data.project_id(),
            bucket_rule_data.token_id(),
            bucket_rule_data.bucket_id(),
            bucket_rule_data.find_one().to_str(),
            bucket_rule_data.find_many().to_str(),
            bucket_rule_data.insert_one(),
            bucket_rule_data.update_one().to_str(),
            bucket_rule_data.delete_one().to_str(),
        ),
    )
}

async fn update_one(
    ctx: web::Data<ApiRestCtx>,
    auth: BearerAuth,
    path: web::Path<UpdateOneBucketRuleReqPath>,
    data: web::Json<UpdateOneBucketRuleReqJson>,
) -> HttpResponse {
    let token = auth.token();

    let token_claim = match ctx.token().jwt().decode(token) {
        Ok(token) => token,
        Err(err) => return Response::error_raw(&StatusCode::BAD_REQUEST, &err.to_string()),
    };

    let admin_id = match token_claim.id() {
        ClaimId::Admin(id) => match AdminDao::db_select(ctx.dao().db(), id).await {
            Ok(data) => *data.id(),
            Err(err) => {
                return Response::error_raw(
                    &StatusCode::UNAUTHORIZED,
                    &format!("Failed to get admin data: {err}"),
                )
            }
        },
        ClaimId::Token(_, _) => {
            return Response::error_raw(
                &StatusCode::BAD_REQUEST,
                "Must be logged in using password-based login",
            )
        }
    };

    let (token_data, mut bucket_rule_data) = match tokio::try_join!(
        TokenDao::db_select(ctx.dao().db(), path.token_id()),
        BucketRuleDao::db_select(ctx.dao().db(), path.rule_id())
    ) {
        Ok(data) => data,
        Err(err) => return Response::error_raw(&StatusCode::BAD_REQUEST, &err.to_string()),
    };

    if token_data.admin_id() != &admin_id {
        return Response::error_raw(&StatusCode::FORBIDDEN, "This token does not belong to you");
    }

    if bucket_rule_data.token_id() != token_data.id() {
        return Response::error_raw(
            &StatusCode::FORBIDDEN,
            "This bucket rule does not belong to you",
        );
    }

    if let Some(find_one) = data.find_one() {
        let find_one = match BucketPermission::from_str(find_one) {
            Ok(rule) => rule,
            Err(err) => return Response::error_raw(&StatusCode::BAD_REQUEST, &err.to_string()),
        };
        bucket_rule_data.set_find_one(&find_one);
    }

    if let Some(find_many) = data.find_many() {
        let find_many = match BucketPermission::from_str(find_many) {
            Ok(rule) => rule,
            Err(err) => return Response::error_raw(&StatusCode::BAD_REQUEST, &err.to_string()),
        };
        bucket_rule_data.set_find_many(&find_many);
    }

    if let Some(insert_one) = data.insert_one() {
        bucket_rule_data.set_insert_one(insert_one);
    }

    if let Some(update_one) = data.update_one() {
        let update_one = match BucketPermission::from_str(update_one) {
            Ok(rule) => rule,
            Err(err) => return Response::error_raw(&StatusCode::BAD_REQUEST, &err.to_string()),
        };
        bucket_rule_data.set_update_one(&update_one);
    }

    if let Some(delete_one) = data.delete_one() {
        let delete_one = match BucketPermission::from_str(delete_one) {
            Ok(rule) => rule,
            Err(err) => return Response::error_raw(&StatusCode::BAD_REQUEST, &err.to_string()),
        };
        bucket_rule_data.set_delete_one(&delete_one);
    }

    if !data.is_all_none() {
        if let Err(err) = bucket_rule_data.db_update(ctx.dao().db()).await {
            return Response::error_raw(&StatusCode::INTERNAL_SERVER_ERROR, &err.to_string());
        }
    }

    Response::data(
        &StatusCode::OK,
        &None,
        &BucketRuleResJson::new(
            bucket_rule_data.id(),
            bucket_rule_data.created_at(),
            bucket_rule_data.updated_at(),
            bucket_rule_data.project_id(),
            bucket_rule_data.token_id(),
            bucket_rule_data.bucket_id(),
            bucket_rule_data.find_one().to_str(),
            bucket_rule_data.find_many().to_str(),
            bucket_rule_data.insert_one(),
            bucket_rule_data.update_one().to_str(),
            bucket_rule_data.delete_one().to_str(),
        ),
    )
}

async fn delete_one(
    ctx: web::Data<ApiRestCtx>,
    auth: BearerAuth,
    path: web::Path<DeleteOneBucketRuleReqPath>,
) -> HttpResponse {
    let token = auth.token();

    let token_claim = match ctx.token().jwt().decode(token) {
        Ok(token) => token,
        Err(err) => return Response::error_raw(&StatusCode::BAD_REQUEST, &err.to_string()),
    };

    let admin_id = match token_claim.id() {
        ClaimId::Admin(id) => match AdminDao::db_select(ctx.dao().db(), id).await {
            Ok(data) => *data.id(),
            Err(err) => {
                return Response::error_raw(
                    &StatusCode::UNAUTHORIZED,
                    &format!("Failed to get admin data: {err}"),
                )
            }
        },
        ClaimId::Token(_, _) => {
            return Response::error_raw(
                &StatusCode::BAD_REQUEST,
                "Must be logged in using password-based login",
            )
        }
    };

    let (token_data, bucket_rule_data) = match tokio::try_join!(
        TokenDao::db_select(ctx.dao().db(), path.token_id()),
        BucketRuleDao::db_select(ctx.dao().db(), path.rule_id())
    ) {
        Ok(data) => data,
        Err(err) => return Response::error_raw(&StatusCode::BAD_REQUEST, &err.to_string()),
    };

    if token_data.admin_id() != &admin_id {
        return Response::error_raw(&StatusCode::FORBIDDEN, "This token does not belong to you");
    }

    if bucket_rule_data.token_id() != token_data.id() {
        return Response::error_raw(
            &StatusCode::FORBIDDEN,
            "This bucket rule does not belong to you",
        );
    }

    if let Err(err) = BucketRuleDao::db_delete(ctx.dao().db(), path.rule_id()).await {
        return Response::error_raw(&StatusCode::INTERNAL_SERVER_ERROR, &err.to_string());
    }

    Response::data(
        &StatusCode::OK,
        &None,
        &DeleteBucketRuleResJson::new(bucket_rule_data.id()),
    )
}

async fn find_many(
    ctx: web::Data<ApiRestCtx>,
    auth: BearerAuth,
    path: web::Path<FindManyBucketRuleReqPath>,
) -> HttpResponse {
    let token = auth.token();

    let token_claim = match ctx.token().jwt().decode(token) {
        Ok(token) => token,
        Err(err) => return Response::error_raw(&StatusCode::BAD_REQUEST, &err.to_string()),
    };

    let admin_id = match token_claim.id() {
        ClaimId::Admin(id) => match AdminDao::db_select(ctx.dao().db(), id).await {
            Ok(data) => *data.id(),
            Err(err) => {
                return Response::error_raw(
                    &StatusCode::UNAUTHORIZED,
                    &format!("Failed to get admin data: {err}"),
                )
            }
        },
        ClaimId::Token(_, _) => {
            return Response::error_raw(
                &StatusCode::BAD_REQUEST,
                "Must be logged in using password-based login",
            )
        }
    };

    let (token_data, bucket_rules_data) = match tokio::try_join!(
        TokenDao::db_select(ctx.dao().db(), path.token_id()),
        BucketRuleDao::db_select_many_by_token_id(ctx.dao().db(), path.token_id())
    ) {
        Ok(data) => data,
        Err(err) => return Response::error_raw(&StatusCode::BAD_REQUEST, &err.to_string()),
    };

    if token_data.admin_id() != &admin_id {
        return Response::error_raw(&StatusCode::FORBIDDEN, "This token does not belong to you");
    }

    Response::data(
        &StatusCode::OK,
        &Some(PaginationRes::new(
            &bucket_rules_data.len(),
            &bucket_rules_data.len(),
        )),
        &bucket_rules_data
            .iter()
            .map(|data| {
                BucketRuleResJson::new(
                    data.id(),
                    data.created_at(),
                    data.updated_at(),
                    data.project_id(),
                    data.token_id(),
                    data.bucket_id(),
                    data.find_one().to_str(),
                    data.find_many().to_str(),
                    data.insert_one(),
                    data.update_one().to_str(),
                    data.delete_one().to_str(),
                )
            })
            .collect::<Vec<_>>(),
    )
}
