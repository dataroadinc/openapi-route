//! OpenAPI document generation from a route catalog.

use std::collections::BTreeMap;

use utoipa::openapi::path::{
    HttpMethod, OperationBuilder, ParameterBuilder, ParameterIn, PathItem,
};
use utoipa::openapi::request_body::RequestBodyBuilder;
use utoipa::openapi::response::ResponseBuilder;
use utoipa::openapi::schema::{ObjectBuilder, Schema, Type};
use utoipa::openapi::tag::TagBuilder;
use utoipa::openapi::{
    Components, Content, ContentBuilder, Info, OpenApi, OpenApiBuilder, PathsBuilder, RefOr,
    Required,
};

use crate::model::{
    ApiCatalog, ContentSpec, Method, ParameterLocation, RequestSpec, ResponseSpec, RouteMetadata,
    RouteParameter,
};

impl ApiCatalog {
    /// Generate an OpenAPI document from the explicit service catalogs.
    pub fn document(&self) -> OpenApi {
        let mut paths = PathsBuilder::new();
        let mut components: BTreeMap<String, RefOr<Schema>> = BTreeMap::new();
        for service in self.services {
            for route in service.routes() {
                let operation = build_operation(route, &mut components);
                paths = paths.path(route.path, path_item(route.method, operation));
            }
        }

        let tags = self
            .services
            .iter()
            .map(|service| {
                TagBuilder::new()
                    .name(service.name)
                    .description(Some(service.description))
                    .build()
            })
            .collect::<Vec<_>>();

        let mut component_registry = Components::new();
        component_registry.schemas.extend(components);

        OpenApiBuilder::new()
            .info(
                Info::builder()
                    .title(self.title)
                    .version(self.version)
                    .build(),
            )
            .paths(paths.build())
            .components(Some(component_registry))
            .tags(Some(tags))
            .build()
    }
}

fn build_operation(
    route: &RouteMetadata,
    components: &mut BTreeMap<String, RefOr<Schema>>,
) -> utoipa::openapi::path::Operation {
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
        operation = operation.parameter(build_parameter(parameter, components));
    }
    if let Some(params_fn) = route.query_params {
        for parameter in params_fn() {
            operation = operation.parameter(parameter);
        }
    }
    if let Some(request) = &route.request {
        operation = operation.request_body(Some(build_request_body(request, components)));
    }
    if route.responses.is_empty() {
        let fallback = ResponseBuilder::new()
            .description("Successful response")
            .build();
        return operation.response("200", fallback).build();
    }
    // Group specs by status so several entries for one status merge
    // their representations instead of the last silently winning.
    let mut seen: Vec<u16> = Vec::new();
    for response in route.responses {
        if seen.contains(&response.status) {
            continue;
        }
        seen.push(response.status);
        let group: Vec<&ResponseSpec> = route
            .responses
            .iter()
            .filter(|candidate| candidate.status == response.status)
            .collect();
        operation = operation.response(
            response.status.to_string(),
            build_response_group(&group, components),
        );
    }
    operation.build()
}

/// Build one response from every spec declared for its status: the
/// first spec's description, the union of all contents.
fn build_response_group(
    group: &[&ResponseSpec],
    components: &mut BTreeMap<String, RefOr<Schema>>,
) -> utoipa::openapi::response::Response {
    let first = group.first().expect("group is non-empty");
    let mut builder = ResponseBuilder::new().description(first.description);
    for spec in group {
        for content in spec.contents {
            builder = builder.content(
                content.media_type,
                build_content(content, spec.type_name, "response", components),
            );
        }
    }
    builder.build()
}

fn build_parameter(
    parameter: &RouteParameter,
    components: &mut BTreeMap<String, RefOr<Schema>>,
) -> utoipa::openapi::path::Parameter {
    let location = match parameter.location {
        ParameterLocation::Path => ParameterIn::Path,
        ParameterLocation::Query => ParameterIn::Query,
        ParameterLocation::Header => ParameterIn::Header,
    };
    let required = if parameter.required || parameter.location == ParameterLocation::Path {
        Required::True
    } else {
        Required::False
    };
    let schema = match parameter.schema {
        Some(schema_fn) => reference_schema(schema_fn, components),
        None => RefOr::T(
            ObjectBuilder::new()
                .schema_type(Type::String)
                .build()
                .into(),
        ),
    };
    let mut builder = ParameterBuilder::new()
        .name(parameter.name)
        .parameter_in(location)
        .required(required)
        .schema(Some(schema));
    if let Some(description) = parameter.description {
        builder = builder.description(Some(description));
    }
    builder.build()
}

fn build_request_body(
    request: &RequestSpec,
    components: &mut BTreeMap<String, RefOr<Schema>>,
) -> utoipa::openapi::request_body::RequestBody {
    let description = request
        .type_name
        .map(|type_name| format!("Request body: {type_name}."));
    let mut builder = RequestBodyBuilder::new()
        .description(description)
        .required(Some(if request.required {
            Required::True
        } else {
            Required::False
        }));
    for content in request.contents {
        builder = builder.content(
            content.media_type,
            build_content(content, request.type_name, "request", components),
        );
    }
    builder.build()
}

fn build_content(
    content: &ContentSpec,
    type_name: Option<&str>,
    direction: &str,
    components: &mut BTreeMap<String, RefOr<Schema>>,
) -> Content {
    let schema = match content.schema {
        Some(schema_fn) => reference_schema(schema_fn, components),
        None => {
            let description = type_name.map_or_else(
                || format!("Untyped {direction} body."),
                |type_name| format!("JSON {direction} body: {type_name}."),
            );
            RefOr::T(
                ObjectBuilder::new()
                    .schema_type(Type::Object)
                    .description(Some(description))
                    .build()
                    .into(),
            )
        }
    };
    let mut builder = ContentBuilder::new().schema(Some(schema));
    if let Some(example) = content.example {
        match serde_json::from_str::<serde_json::Value>(example) {
            Ok(value) => builder = builder.example(Some(value)),
            Err(error) => panic!("route metadata example is not valid JSON: {error}: {example}"),
        }
    }
    builder.build()
}

/// Resolve a schema function into a `$ref`, registering its component
/// schemas. The first registration of a component name wins.
fn reference_schema(
    schema_fn: crate::schema::SchemaFn,
    components: &mut BTreeMap<String, RefOr<Schema>>,
) -> RefOr<Schema> {
    let set = schema_fn();
    for (name, schema) in set.components {
        components.entry(name).or_insert(schema);
    }
    set.reference
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
