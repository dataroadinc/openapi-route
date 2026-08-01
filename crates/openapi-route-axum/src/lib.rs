//! Axum and Swagger UI integration for the openapi-route metadata crate.

use axum::Router;
use openapi_route::ApiCatalog;
use utoipa_swagger_ui::SwaggerUi;

/// Mount Swagger UI and the generated OpenAPI document.
pub fn router(catalog: &'static ApiCatalog) -> Router {
    Router::new().merge(SwaggerUi::new("/swagger-ui").url("/openapi.json", catalog.document()))
}
