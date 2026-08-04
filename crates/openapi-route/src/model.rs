//! Route, service, and catalog metadata types.

use crate::schema::{ParamsFn, SchemaFn};

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

/// Where a documented parameter is carried.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParameterLocation {
    /// A `{placeholder}` in the path template.
    Path,
    /// A query-string parameter.
    Query,
    /// A request header.
    Header,
}

/// A documented operation parameter.
#[derive(Clone, Copy, Debug)]
pub struct RouteParameter {
    /// Parameter name as it appears between braces in the path (or
    /// the query/header name).
    pub name: &'static str,
    /// Human-readable parameter description.
    pub description: Option<&'static str>,
    /// Where the parameter is carried.
    pub location: ParameterLocation,
    /// Whether the parameter must be present. Path parameters are
    /// always required.
    pub required: bool,
    /// Typed schema; `None` documents a plain string.
    pub schema: Option<SchemaFn>,
}

/// One representation of a request or response body.
#[derive(Clone, Copy, Debug)]
pub struct ContentSpec {
    /// The representation's media type.
    pub media_type: &'static str,
    /// Typed schema; `None` falls back to a prose object description
    /// from the owning spec's `type_name`.
    pub schema: Option<SchemaFn>,
    /// Example value as a JSON document.
    pub example: Option<&'static str>,
}

impl ContentSpec {
    /// A JSON representation described only by the owning spec's type
    /// name — the shape every pre-typed annotation lowers to.
    pub const PROSE_JSON: Self = Self {
        media_type: "application/json",
        schema: None,
        example: None,
    };
}

/// A documented request body.
#[derive(Clone, Copy, Debug)]
pub struct RequestSpec {
    /// Body type name for prose fallbacks.
    pub type_name: Option<&'static str>,
    /// Whether a body is required.
    pub required: bool,
    /// Accepted representations.
    pub contents: &'static [ContentSpec],
}

/// A documented response for one status code.
#[derive(Clone, Copy, Debug)]
pub struct ResponseSpec {
    /// HTTP status code.
    pub status: u16,
    /// Human-readable response description.
    pub description: &'static str,
    /// Body type name for prose fallbacks.
    pub type_name: Option<&'static str>,
    /// Offered representations; empty documents a bodyless response.
    pub contents: &'static [ContentSpec],
}

/// A documented HTTP operation.
#[derive(Clone, Copy, Debug)]
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
    /// Individually declared parameters.
    pub parameters: &'static [RouteParameter],
    /// Derived query parameters of a parameter struct, when the
    /// operation has one.
    pub query_params: Option<ParamsFn>,
    /// Request body, when the operation accepts one.
    pub request: Option<RequestSpec>,
    /// Success and error responses. Empty documents a generic 200.
    pub responses: &'static [ResponseSpec],
}

/// A named group of routes owned by one HTTP service.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ApiService {
    /// Service name used in the generated document.
    pub name: &'static str,
    /// Service description used for its OpenAPI tag.
    pub description: &'static str,
}

/// A route registered with an [`ApiService`] by an annotated handler.
#[derive(Clone, Copy, Debug)]
pub struct RegisteredRoute {
    /// Service that owns the route.
    pub service: &'static ApiService,
    /// Handler-generated route metadata.
    pub route: &'static RouteMetadata,
}

inventory::collect!(RegisteredRoute);

/// Register manually declared route metadata with a service.
#[macro_export]
macro_rules! register_route {
    ($service:path, $route:expr) => {
        $crate::inventory::submit! {
            $crate::RegisteredRoute {
                service: &$service,
                route: &$route,
            }
        }
    };
}

impl ApiService {
    /// Iterate over route metadata registered with this service.
    pub fn routes(&self) -> impl Iterator<Item = &'static RouteMetadata> {
        inventory::iter::<RegisteredRoute>
            .into_iter()
            .filter(move |registration| std::ptr::eq(registration.service, self))
            .map(|registration| registration.route)
    }
}

/// The complete route catalog used to generate one OpenAPI document.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ApiCatalog {
    /// OpenAPI title.
    pub title: &'static str,
    /// Browser title for the Swagger UI page.
    pub ui_title: &'static str,
    /// OpenAPI version.
    pub version: &'static str,
    /// Services included in the document.
    pub services: &'static [&'static ApiService],
}
