use std::str::FromStr;

use actix_web::{http::StatusCode, web, HttpResponse};
use futures::future;
use hb_dao::{
    collection::{CollectionDao, SchemaFieldKind, SchemaFieldModel},
    project::ProjectDao,
    Db,
};
use hb_token_jwt::kind::JwtTokenKind;

use crate::{
    context::ApiRestContext as Context,
    v1::model::{
        collection::{
            CollectionResJson, DeleteCollectionResJson, DeleteOneCollectionReqPath,
            FindOneCollectionReqPath, InsertOneCollectionReqJson, InsertOneCollectionReqPath,
            SchemaFieldModelJson, UpdateOneCollectionReqJson, UpdateOneCollectionReqPath,
        },
        PaginationRes, Response, TokenReqHeader,
    },
};

pub fn collection_api(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/project/{project_id}/collection")
            .route("", web::post().to(insert_one))
            .route("/{collection_id}", web::get().to(find_one))
            .route("/{collection_id}", web::patch().to(update_one))
            .route("/{collection_id}", web::patch().to(delete_one)),
    );

    cfg.service(
        web::scope("/project/{project_id}/collections").route("", web::get().to(find_many)),
    );
}

async fn insert_one(
    ctx: web::Data<Context>,
    token: web::Header<TokenReqHeader>,
    path: web::Path<InsertOneCollectionReqPath>,
    data: web::Json<InsertOneCollectionReqJson>,
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

    let mut schema_fields = Vec::new();
    for field in data.schema_fields().iter() {
        schema_fields.push(SchemaFieldModel::new(
            field.name(),
            &match SchemaFieldKind::from_str(field.kind()) {
                Ok(kind) => kind,
                Err(err) => {
                    return Response::error(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        err.to_string().as_str(),
                    )
                }
            },
            field.required(),
        ))
    }

    let collection_data = CollectionDao::new(
        path.project_id(),
        data.name(),
        schema_fields.as_ref(),
        data.indexes(),
    );

    if let Err(err) = collection_data.insert(&db).await {
        return Response::error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string().as_str());
    }

    Response::data(
        StatusCode::CREATED,
        None,
        CollectionResJson::new(
            collection_data.id(),
            collection_data.created_at(),
            collection_data.updated_at(),
            collection_data.project_id(),
            collection_data.name(),
            collection_data
                .schema_fields()
                .iter()
                .map(|field| {
                    SchemaFieldModelJson::new(
                        field.name(),
                        field.kind().to_string().as_str(),
                        field.required(),
                    )
                })
                .collect::<Vec<_>>()
                .as_ref(),
            collection_data.indexes(),
        ),
    )
}

async fn find_one(
    ctx: web::Data<Context>,
    token: web::Header<TokenReqHeader>,
    path: web::Path<FindOneCollectionReqPath>,
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

    let (project_data, collection_data) = match tokio::try_join!(
        ProjectDao::select(&db, path.project_id()),
        CollectionDao::select(&db, path.collection_id()),
    ) {
        Ok(data) => data,
        Err(err) => return Response::error(StatusCode::BAD_REQUEST, err.to_string().as_str()),
    };

    if project_data.id() != collection_data.project_id() {
        return Response::error(StatusCode::BAD_REQUEST, "Project ID does not match");
    }

    Response::data(
        StatusCode::OK,
        None,
        CollectionResJson::new(
            collection_data.id(),
            collection_data.created_at(),
            collection_data.updated_at(),
            collection_data.project_id(),
            collection_data.name(),
            collection_data
                .schema_fields()
                .iter()
                .map(|field| {
                    SchemaFieldModelJson::new(
                        field.name(),
                        field.kind().to_string().as_str(),
                        field.required(),
                    )
                })
                .collect::<Vec<_>>()
                .as_ref(),
            collection_data.indexes(),
        ),
    )
}

async fn update_one(
    ctx: web::Data<Context>,
    token: web::Header<TokenReqHeader>,
    path: web::Path<UpdateOneCollectionReqPath>,
    data: web::Json<UpdateOneCollectionReqJson>,
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

    let (project_data, mut collection_data) = match tokio::try_join!(
        ProjectDao::select(&db, path.project_id()),
        CollectionDao::select(&db, path.collection_id()),
    ) {
        Ok(data) => data,
        Err(err) => return Response::error(StatusCode::BAD_REQUEST, err.to_string().as_str()),
    };

    if project_data.id() != collection_data.project_id() {
        return Response::error(StatusCode::BAD_REQUEST, "Project ID does not match");
    }

    if let Some(name) = data.name() {
        collection_data.set_name(name);
    }

    if let Some(schema_field) = data.schema_fields() {
        let mut schema_fields = Vec::new();
        for field in schema_field.iter() {
            schema_fields.push(SchemaFieldModel::new(
                field.name(),
                match &SchemaFieldKind::from_str(field.kind()) {
                    Ok(kind) => kind,
                    Err(err) => {
                        return Response::error(
                            StatusCode::INTERNAL_SERVER_ERROR,
                            err.to_string().as_str(),
                        )
                    }
                },
                field.required(),
            ));
        }
        collection_data.set_schema_fields(&schema_fields);
    }

    if let Some(indexes) = data.indexes() {
        collection_data.set_indexes(indexes);
    }

    if !data.is_all_none() {
        if let Err(err) = collection_data.update(&db).await {
            return Response::error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string().as_str());
        }
    }

    Response::data(
        StatusCode::OK,
        None,
        CollectionResJson::new(
            collection_data.id(),
            collection_data.created_at(),
            collection_data.updated_at(),
            collection_data.project_id(),
            collection_data.name(),
            collection_data
                .schema_fields()
                .iter()
                .map(|field| {
                    SchemaFieldModelJson::new(
                        field.name(),
                        field.kind().to_string().as_str(),
                        field.required(),
                    )
                })
                .collect::<Vec<_>>()
                .as_ref(),
            collection_data.indexes(),
        ),
    )
}

async fn delete_one(
    ctx: web::Data<Context>,
    token: web::Header<TokenReqHeader>,
    path: web::Path<DeleteOneCollectionReqPath>,
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

    let (project_data, collection_data) = match tokio::try_join!(
        ProjectDao::select(&db, path.project_id()),
        CollectionDao::select(&db, path.collection_id()),
    ) {
        Ok(data) => data,
        Err(err) => return Response::error(StatusCode::BAD_REQUEST, err.to_string().as_str()),
    };

    if project_data.id() != collection_data.project_id() {
        return Response::error(StatusCode::BAD_REQUEST, "Project ID does not match");
    }

    if let Err(err) = CollectionDao::delete(&db, path.collection_id()).await {
        return Response::error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string().as_str());
    }

    Response::data(
        StatusCode::OK,
        None,
        DeleteCollectionResJson::new(collection_data.id()),
    )
}

async fn find_many(
    ctx: web::Data<Context>,
    token: web::Header<TokenReqHeader>,
    path: web::Path<DeleteOneCollectionReqPath>,
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

    let collections_data = match CollectionDao::select_by_project_id(&db, path.project_id()).await {
        Ok(data) => data,
        Err(err) => return Response::error(StatusCode::BAD_REQUEST, err.to_string().as_str()),
    };

    if let Err(err) = future::try_join_all(collections_data.iter().map(|collection| async {
        let project_data = match ProjectDao::select(&db, collection.project_id()).await {
            Ok(data) => data,
            Err(err) => {
                return Err(Response::error(
                    StatusCode::BAD_REQUEST,
                    err.to_string().as_str(),
                ))
            }
        };

        if project_data.admin_id() != token_claim.id() {
            return Err(Response::error(
                StatusCode::FORBIDDEN,
                "Some collections do not belong to you",
            ));
        }

        Ok(())
    }))
    .await
    {
        return err;
    }

    Response::data(
        StatusCode::OK,
        Some(PaginationRes::new(
            &(collections_data.len() as i64),
            &(collections_data.len() as i64),
            &1,
            &(collections_data.len() as i64),
        )),
        collections_data
            .iter()
            .map(|data| {
                CollectionResJson::new(
                    data.id(),
                    data.created_at(),
                    data.updated_at(),
                    data.project_id(),
                    data.name(),
                    data.schema_fields()
                        .iter()
                        .map(|field| {
                            SchemaFieldModelJson::new(
                                field.name(),
                                field.kind().to_string().as_str(),
                                field.required(),
                            )
                        })
                        .collect::<Vec<_>>()
                        .as_ref(),
                    data.indexes(),
                )
            })
            .collect::<Vec<_>>(),
    )
}
