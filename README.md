# openapi-route

Handler-local OpenAPI metadata for Rust HTTP services.

The project is split into three crates:

- openapi-route contains framework-neutral route metadata and document generation.
- openapi-route-macros generates explicit route constants from handler annotations.
- openapi-route-axum mounts the generated document and Swagger UI in Axum.

Route metadata is assembled through explicit service-owned catalogs. The library does
not use a process-global registry or constructor-based registration.

