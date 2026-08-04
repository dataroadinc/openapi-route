//! Framework-neutral route metadata and OpenAPI document generation.
//!
//! Handlers declare explicit, `'static` route metadata — typed
//! parameters, request/response specs with per-media-type schemas and
//! examples, and typed error responses — and one catalog call
//! generates the OpenAPI document with `$ref`-reusable component
//! schemas.

pub use inventory;
pub use openapi_route_macros::openapi_handler;

mod document;
mod model;
mod schema;

pub use model::{
    ApiCatalog, ApiService, ContentSpec, Method, ParameterLocation, RegisteredRoute, RequestSpec,
    ResponseSpec, RouteMetadata, RouteParameter,
};
pub use schema::{NamedSchemaSet, ParamsFn, SchemaFn, query_params, schema_set};
