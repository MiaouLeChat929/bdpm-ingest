use utoipa::OpenApi;
use crate::api;

#[derive(OpenApi)]
#[openapi(
    paths(
        api::search::search_drugs,
        api::drugs::drug_detail,
        api::groups::list_generic_groups,
        api::groups::generic_group_detail,
        api::atc::atc_top_level,
        api::atc::atc_detail,
        api::availability::availability,
        api::health,
    ),
    components(schemas(
        api::search::DrugSearchResult,
        api::drugs::DrugDetail,
        api::drugs::Presentation,
        api::drugs::Composition,
        api::groups::GenericGroupList,
        api::groups::GenericGroupMember,
        api::atc::AtcCode,
        api::atc::AtcDetail,
        api::availability::AvailabilityRow,
        api::HealthResponse,
    )),
    tags(
        (name = "bdpm-ingest", description = "BDPM Drug Database API")
    )
)]
pub struct ApiDoc;

/// Serve the OpenAPI spec as JSON
pub async fn openapi_json() -> impl axum::response::IntoResponse {
    let json = ApiDoc::openapi().to_json().unwrap_or_default();
    ([("content-type", "application/json")], json)
}

/// Serve the OpenAPI spec as YAML
pub async fn openapi_yaml() -> impl axum::response::IntoResponse {
    // utoipa 4.x supports to_yaml via utoipa-gen
    let yaml = serde_yaml::to_string(&ApiDoc::openapi()).unwrap_or_default();
    ([("content-type", "application/x-yaml")], yaml)
}