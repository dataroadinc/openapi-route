//! Framework-neutral route metadata and OpenAPI document generation.

pub use openapi_route_macros::openapi_handler;

use utoipa::openapi::path::{
    HttpMethod, OperationBuilder, ParameterBuilder, ParameterIn, PathItem,
};
use utoipa::openapi::request_body::RequestBodyBuilder;
use utoipa::openapi::response::ResponseBuilder;
use utoipa::openapi::schema::{ObjectBuilder, Type};
use utoipa::openapi::{Content, Info, OpenApi, OpenApiBuilder, PathsBuilder, RefOr, Required};

/// HTTP methods understood by the route metadata layer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Method {
    /// GET.
    Get,
    /// POST.
    Post,
    /// PUT.
    Put,
    /// PATCH.
    Patch,
    /// DELETE.
    Delete,
    /// HEAD.
    Head,
}

/// A documented path parameter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RouteParameter {
    /// Parameter name as it appears between braces in the path.
    pub name: &'static str,
    /// Human-readable parameter description.
    pub description: Option<&'static str>,
}

/// A documented HTTP operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RouteMetadata {
    /// HTTP method.
    pub method: Method,
    /// OpenAPI path template.
    pub path: &'static str,
    /// Stable operation identifier.
    pub operation_id: &'static str,
    /// Short operation summary.
    pub summary: &'static str,
    /// Long operation description.
    pub description: Option<&'static str>,
    /// OpenAPI tags.
    pub tags: &'static [&'static str],
    /// Path parameters.
    pub parameters: &'static [RouteParameter],
    /// Name of the request body type, when the handler accepts JSON.
    pub request_type: Option<&'static str>,
    /// Name of the successful response type, when known.
    pub response_type: Option<&'static str>,
    /// Names of handler error types.
    pub error_types: &'static [&'static str],
}

/// A named group of routes owned by one HTTP service.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ApiService {
    /// Service name used in the generated document.
    pub name: &'static str,
    /// Service description used for its OpenAPI tag.
    pub description: &'static str,
    /// Explicit service route catalog.
    pub routes: &'static [RouteMetadata],
}

/// The complete route catalog used to generate one OpenAPI document.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ApiCatalog {
    /// OpenAPI title.
    pub title: &'static str,
    /// OpenAPI version.
    pub version: &'static str,
    /// Services included in the document.
    pub services: &'static [ApiService],
}

impl ApiCatalog {
    /// Generate an OpenAPI document from the explicit service catalogs.
    pub fn document(&self) -> OpenApi {
        let mut paths = PathsBuilder::new();
        for service in self.services {
            for route in service.routes {
                let mut operation = OperationBuilder::new()
                    .operation_id(Some(route.operation_id))
                    .summary(Some(route.summary));
                if let Some(description) = route.description {
                    operation = operation.description(Some(description));
                }
                for tag in route.tags {
                    operation = operation.tag(*tag);
                }
                for parameter in route.parameters {
                    let mut builder = ParameterBuilder::new()
                        .name(parameter.name)
                        .parameter_in(ParameterIn::Path)
                        .required(Required::True)
                        .schema(Some(RefOr::T(
                            ObjectBuilder::new()
                                .schema_type(Type::String)
                                .build()
                                .into(),
                        )));
                    if let Some(description) = parameter.description {
                        builder = builder.description(Some(description));
                    }
                    operation = operation.parameter(builder.build());
                }
                if let Some(request_type) = route.request_type {
                    let schema = ObjectBuilder::new()
                        .schema_type(Type::Object)
                        .description(Some(format!("JSON request body: {request_type}.")))
                        .build();
                    operation = operation.request_body(Some(
                        RequestBodyBuilder::new()
                            .description(Some(format!("Request body: {request_type}.")))
                            .required(Some(Required::True))
                            .content("application/json", Content::new(Some(schema)))
                            .build(),
                    ));
                }
                let success_description = route.response_type.map_or_else(
                    || "Successful response".to_owned(),
                    |response_type| format!("Successful response containing {response_type}."),
                );
                let success_response = ResponseBuilder::new().description(success_description);
                let success_response = match route.response_type {
                    Some(response_type) => success_response.content(
                        "application/json",
                        Content::new(Some(
                            ObjectBuilder::new()
                                .schema_type(Type::Object)
                                .description(Some(format!(
                                    "JSON response body: {response_type}."
                                )))
                                .build(),
                        )),
                    ),
                    None => success_response,
                };
                let mut operation = operation.response("200", success_response.build());
                if !route.error_types.is_empty() {
                    let error_description = format!(
                        "Request failed with one of: {}.",
                        route.error_types.join(", "),
                    );
                    operation = operation.response(
                        "400",
                        ResponseBuilder::new()
                            .description(error_description)
                            .build(),
                    );
                }
                let operation = operation.build();
                paths = paths.path(route.path, path_item(route.method, operation));
            }
        }

        OpenApiBuilder::new()
            .info(
                Info::builder()
                    .title(self.title)
                    .version(self.version)
                    .build(),
            )
            .paths(paths.build())
            .build()
    }
}

fn path_item(method: Method, operation: utoipa::openapi::path::Operation) -> PathItem {
    match method {
        Method::Get => PathItem::new(HttpMethod::Get, operation),
        Method::Post => PathItem::new(HttpMethod::Post, operation),
        Method::Put => PathItem::new(HttpMethod::Put, operation),
        Method::Patch => PathItem::new(HttpMethod::Patch, operation),
        Method::Delete => PathItem::new(HttpMethod::Delete, operation),
        Method::Head => PathItem::new(HttpMethod::Head, operation),
    }
}
