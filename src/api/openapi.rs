use utoipa::OpenApi;

// ApiDoc lives here so main.rs can import it for dump-open-api.
// Uses relative paths since this file is inside the `api` module.
#[derive(OpenApi)]
#[openapi(
    paths(
        super::search::search_drugs,
        super::drugs::drug_detail,
        super::groups::list_generic_groups,
        super::groups::generic_group_detail,
        super::atc::atc_top_level,
        super::atc::atc_detail,
        super::availability::availability,
        super::health,
        super::safety::drug_safety,
    ),
    components(schemas(
        super::search::DrugSearchResult,
        super::drugs::DrugDetail,
        super::drugs::Presentation,
        super::drugs::Composition,
        super::groups::GenericGroupList,
        super::groups::GenericGroupMember,
        super::atc::AtcCode,
        super::atc::AtcDetail,
        super::availability::AvailabilityRow,
        super::safety::SafetyResponse,
        super::safety::SafetyAlert,
        super::HealthResponse,
    )),
    tags(
        (name = "bdpm-ingest", description = "BDPM Drug Database API")
    )
)]
pub struct ApiDoc;

/// Serve the OpenAPI spec as JSON — generated dynamically.
pub async fn openapi_json() -> impl axum::response::IntoResponse {
    let json = ApiDoc::openapi().to_json().unwrap_or_default();
    ([("content-type", "application/json")], json)
}

/// Serve the OpenAPI spec as YAML — generated dynamically (utoipa yaml feature required).
pub async fn openapi_yaml() -> impl axum::response::IntoResponse {
    let yaml = ApiDoc::openapi().to_yaml().unwrap_or_default();
    ([("content-type", "application/x-yaml")], yaml)
}
