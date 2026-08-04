//! Document-generation contract tests for typed route metadata.

use openapi_route::{
    ApiCatalog, ApiService, ContentSpec, Method, ParameterLocation, RequestSpec, ResponseSpec,
    RouteMetadata, RouteParameter, register_route, schema_set,
};
use serde_json::Value;
use utoipa::ToSchema;

#[derive(ToSchema)]
#[allow(dead_code)]
struct CreateWidget {
    name: String,
    size: WidgetSize,
}

#[derive(ToSchema)]
#[allow(dead_code)]
struct WidgetSize {
    width: u32,
    height: u32,
}

#[derive(ToSchema)]
#[allow(dead_code)]
struct WidgetError {
    code: String,
    message: String,
}

static SERVICE: ApiService = ApiService {
    name: "widgets",
    description: "Widget management operations.",
};

static TYPED_ROUTE: RouteMetadata = RouteMetadata {
    method: Method::Post,
    path: "/api/v1/widget/{slug}",
    operation_id: "create_widget",
    summary: "Create a widget",
    description: Some("Creates a widget under the given slug."),
    tags: &["Widgets"],
    parameters: &[
        RouteParameter {
            name: "slug",
            description: Some("Stable widget slug."),
            location: ParameterLocation::Path,
            required: true,
            schema: None,
        },
        RouteParameter {
            name: "dry_run",
            description: Some("Validate without persisting."),
            location: ParameterLocation::Query,
            required: false,
            schema: None,
        },
    ],
    query_params: None,
    request: Some(RequestSpec {
        type_name: Some("CreateWidget"),
        required: true,
        contents: &[ContentSpec {
            media_type: "application/json",
            schema: Some(schema_set::<CreateWidget>),
            example: Some(r#"{"name":"gear","size":{"width":3,"height":4}}"#),
        }],
    }),
    responses: &[
        ResponseSpec {
            status: 201,
            description: "Widget created.",
            type_name: Some("CreateWidget"),
            contents: &[
                ContentSpec {
                    media_type: "application/json",
                    schema: Some(schema_set::<CreateWidget>),
                    example: None,
                },
                ContentSpec {
                    media_type: "application/vnd.example.envelope+json",
                    schema: None,
                    example: None,
                },
            ],
        },
        ResponseSpec {
            status: 422,
            description: "Widget rejected.",
            type_name: Some("WidgetError"),
            contents: &[ContentSpec {
                media_type: "application/json",
                schema: Some(schema_set::<WidgetError>),
                example: None,
            }],
        },
    ],
};

static LEGACY_ROUTE: RouteMetadata = RouteMetadata {
    method: Method::Get,
    path: "/api/v1/widgets",
    operation_id: "list_widgets",
    summary: "List widgets",
    description: None,
    tags: &["Widgets"],
    parameters: &[],
    query_params: None,
    request: None,
    responses: &[],
};

static CATALOG: ApiCatalog = ApiCatalog {
    title: "Widget API",
    ui_title: "Widget APIs",
    version: "1.2.3",
    services: &[&SERVICE],
};

register_route!(SERVICE, TYPED_ROUTE);
register_route!(SERVICE, LEGACY_ROUTE);

fn document() -> Value {
    serde_json::to_value(CATALOG.document()).expect("document serializes")
}

#[test]
fn components_are_registered_and_referenced() {
    let document = document();
    let schemas = &document["components"]["schemas"];
    assert!(schemas.get("CreateWidget").is_some(), "{schemas}");
    assert!(
        schemas.get("WidgetSize").is_some(),
        "nested dependency registered: {schemas}"
    );
    assert!(schemas.get("WidgetError").is_some(), "{schemas}");

    let request_schema = &document["paths"]["/api/v1/widget/{slug}"]["post"]["requestBody"]["content"]
        ["application/json"]["schema"];
    assert_eq!(
        request_schema["$ref"],
        Value::String("#/components/schemas/CreateWidget".to_owned()),
    );
}

#[test]
fn parameters_carry_descriptions_and_locations() {
    let document = document();
    let parameters = document["paths"]["/api/v1/widget/{slug}"]["post"]["parameters"]
        .as_array()
        .expect("parameters array")
        .clone();
    let slug = parameters
        .iter()
        .find(|parameter| parameter["name"] == "slug")
        .expect("slug parameter");
    assert_eq!(slug["in"], "path");
    assert_eq!(slug["required"], Value::Bool(true));
    assert_eq!(slug["description"], "Stable widget slug.");
    let dry_run = parameters
        .iter()
        .find(|parameter| parameter["name"] == "dry_run")
        .expect("dry_run parameter");
    assert_eq!(dry_run["in"], "query");
    assert_eq!(dry_run["required"], Value::Bool(false));
}

#[test]
fn responses_carry_status_media_types_and_examples() {
    let document = document();
    let operation = &document["paths"]["/api/v1/widget/{slug}"]["post"];
    let created = &operation["responses"]["201"];
    assert_eq!(created["description"], "Widget created.");
    assert!(created["content"]["application/json"]["schema"]["$ref"].is_string());
    assert!(
        created["content"]["application/vnd.example.envelope+json"].is_object(),
        "second media type documented",
    );
    let rejected = &operation["responses"]["422"];
    assert_eq!(
        rejected["content"]["application/json"]["schema"]["$ref"],
        Value::String("#/components/schemas/WidgetError".to_owned()),
    );

    let example = &operation["requestBody"]["content"]["application/json"]["example"];
    assert_eq!(example["name"], "gear");
    assert_eq!(example["size"]["width"], 3);
}

#[test]
fn legacy_metadata_still_documents_a_generic_200() {
    let document = document();
    let operation = &document["paths"]["/api/v1/widgets"]["get"];
    assert_eq!(
        operation["responses"]["200"]["description"],
        "Successful response",
    );
}

#[test]
fn services_emit_tag_descriptions() {
    let document = document();
    let tags = document["tags"].as_array().expect("tags array").clone();
    let widget_tag = tags
        .iter()
        .find(|tag| tag["name"] == "widgets")
        .expect("widgets tag");
    assert_eq!(widget_tag["description"], "Widget management operations.");
}
