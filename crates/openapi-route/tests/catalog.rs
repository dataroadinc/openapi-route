use openapi_route::{ApiCatalog, Method, openapi_handler};

struct Json<T>(T);
struct Request;
struct Response;
struct Error;

static API_SERVICE: openapi_route::ApiService = openapi_route::ApiService {
    name: "tasks",
    description: "Task operations",
};

#[allow(dead_code)]
#[openapi_handler(
    service = API_SERVICE,
    method = "GET",
    path = "/tasks/{id}",
    operation_id = "get_task",
    tag = "tasks",
    parameter = "id",
    request_type = "TaskRequest",
    response_type = "Task",
    error_type = "TaskError"
)]
/// `GET /tasks/{id}` — Get task.
///
/// Returns one task.
fn get_task() {}

#[allow(dead_code)]
#[openapi_handler(method = "POST", path = "/tasks")]
fn create_task(_body: Json<Request>) -> Result<Json<Response>, Error> {
    panic!("test handler is never called")
}

static CATALOG: ApiCatalog = ApiCatalog {
    title: "Test API",
    version: "0.1.0",
    services: &[&API_SERVICE],
};

#[test]
fn macro_generates_explicit_route_metadata() {
    let route = &OPENAPI_ROUTE_GET_TASK;
    assert_eq!(route.method, Method::Get);
    assert_eq!(route.path, "/tasks/{id}");
    assert_eq!(route.operation_id, "get_task");
    assert_eq!(route.summary, "Get task.");
    assert_eq!(route.description, Some("Returns one task."));
    assert_eq!(route.tags, &["tasks"]);
    assert_eq!(route.parameters[0].name, "id");
    assert_eq!(route.request_type, Some("TaskRequest"));
    assert_eq!(route.response_type, Some("Task"));
    assert_eq!(route.error_types, &["TaskError"]);
}

#[test]
fn catalog_generates_openapi_paths_without_global_registration() {
    let document = CATALOG.document();
    let path = document.paths.paths.get("/tasks/{id}").expect("path");
    let operation = path.get.as_ref().expect("GET operation");
    assert!(operation.request_body.is_some());
    assert!(operation
        .responses
        .responses
        .get("200")
        .and_then(|response| match response {
            utoipa::openapi::RefOr::T(response) => Some(&response.content),
            utoipa::openapi::RefOr::Ref(_) => None,
        })
        .and_then(|content| content.get("application/json"))
        .is_some());
    assert!(operation.responses.responses.contains_key("400"));
}

#[test]
fn catalog_collects_routes_registered_for_each_service() {
    let document = CATALOG.document();
    assert!(document.paths.paths.contains_key("/tasks"));
    assert!(document.paths.paths.contains_key("/tasks/{id}"));
}

#[test]
fn macro_infers_json_and_result_types() {
    let route = &OPENAPI_ROUTE_CREATE_TASK;
    assert_eq!(route.request_type, Some("Request"));
    assert_eq!(route.response_type, Some("Json<Response>"));
    assert_eq!(route.error_types, &["Error"]);
}
