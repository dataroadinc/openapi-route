use openapi_route::{ApiCatalog, ApiService, Method, openapi_handler};

#[allow(dead_code)]
#[openapi_handler(
    method = "GET",
    path = "/tasks/{id}",
    operation_id = "get_task",
    tag = "tasks",
    parameter = "id"
)]
/// Get task.
///
/// Returns one task.
fn get_task() {}

static ROUTES: &[openapi_route::RouteMetadata] = &[OPENAPI_ROUTE_GET_TASK];

static SERVICE: ApiService = ApiService {
    name: "tasks",
    description: "Task operations",
    routes: ROUTES,
};

static CATALOG: ApiCatalog = ApiCatalog {
    title: "Test API",
    version: "0.1.0",
    services: &[SERVICE],
};

#[test]
fn macro_generates_explicit_route_metadata() {
    let route = &ROUTES[0];
    assert_eq!(route.method, Method::Get);
    assert_eq!(route.path, "/tasks/{id}");
    assert_eq!(route.operation_id, "get_task");
    assert_eq!(route.summary, "Get task.");
    assert_eq!(route.description, Some("Returns one task."));
    assert_eq!(route.tags, &["tasks"]);
    assert_eq!(route.parameters[0].name, "id");
}

#[test]
fn catalog_generates_openapi_paths_without_global_registration() {
    let document = CATALOG.document();
    assert!(document.paths.paths.contains_key("/tasks/{id}"));
}
