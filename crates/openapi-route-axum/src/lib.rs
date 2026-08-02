//! Axum and Swagger UI integration for the openapi-route metadata crate.

use axum::Router;
use axum::response::{Html, IntoResponse, Json};
use axum::routing::get;
use openapi_route::ApiCatalog;

/// Mount Swagger UI and the generated OpenAPI document.
pub fn router<S>(catalog: &'static ApiCatalog) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::<S>::new()
        .route(
            "/openapi.json",
            get(move || async move { Json(catalog.document()) }),
        )
        .route(
            "/swagger-ui",
            get({
                let title = catalog.ui_title;
                move || async move { swagger_ui(title) }
            }),
        )
        .route(
            "/swagger-ui/",
            get({
                let title = catalog.ui_title;
                move || async move { swagger_ui(title) }
            }),
        )
}

fn swagger_ui(title: &str) -> impl IntoResponse {
    Html(swagger_html(title))
}

fn swagger_html(title: &str) -> String {
    r#"<!DOCTYPE html>
<html>
<head>
    <title>__OPENAPI_ROUTE_UI_TITLE__</title>
    <link rel="stylesheet" type="text/css" href="https://unpkg.com/swagger-ui-dist@5.9.0/swagger-ui.css" />
    <style>
        html { box-sizing: border-box; overflow-y: scroll; }
        *, *:before, *:after { box-sizing: inherit; }
        body { margin: 0; background: #fafafa; }
    </style>
</head>
<body>
    <div id="swagger-ui"></div>
    <script src="https://unpkg.com/swagger-ui-dist@5.9.0/swagger-ui-bundle.js"></script>
    <script src="https://unpkg.com/swagger-ui-dist@5.9.0/swagger-ui-standalone-preset.js"></script>
    <script>
        window.onload = function() {
            window.ui = SwaggerUIBundle({
                url: '/openapi.json',
                dom_id: '#swagger-ui',
                deepLinking: true,
                presets: [
                    SwaggerUIBundle.presets.apis,
                    SwaggerUIStandalonePreset
                ],
                plugins: [SwaggerUIBundle.plugins.DownloadUrl],
                layout: 'StandaloneLayout'
            });
        };
    </script>
</body>
</html>"#
        .replace("__OPENAPI_ROUTE_UI_TITLE__", title)
}

#[cfg(test)]
mod tests {
    use super::swagger_html;

    #[test]
    fn swagger_html_uses_catalog_ui_title() {
        let html = swagger_html("WWKG Gateway APIs");
        assert!(html.contains("<title>WWKG Gateway APIs</title>"));
    }
}
