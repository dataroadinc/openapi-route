//! Schema and parameter providers bridging `utoipa` derive types into
//! `'static` route metadata.
//!
//! Route metadata lives in statics, so schemas are carried as function
//! pointers. `schema_set::<T>` and `query_params::<T>` are generic
//! functions that coerce to those pointers when instantiated with a
//! concrete type.

use utoipa::openapi::path::{Parameter, ParameterIn};
use utoipa::openapi::schema::Schema;
use utoipa::openapi::{Ref, RefOr};

/// Produces the named schema set for one body or parameter type.
pub type SchemaFn = fn() -> NamedSchemaSet;

/// Produces the documented query/header/path parameters of one
/// parameter struct.
pub type ParamsFn = fn() -> Vec<Parameter>;

/// A `$ref` to a named component schema plus every component schema
/// (the type itself and its nested dependencies) that the document
/// must register to make the reference resolvable.
pub struct NamedSchemaSet {
    /// Reference to the root schema, pointing into
    /// `#/components/schemas`.
    pub reference: RefOr<Schema>,
    /// Named component schemas to merge into the document.
    pub components: Vec<(String, RefOr<Schema>)>,
}

/// Build the [`NamedSchemaSet`] of a [`utoipa::ToSchema`] type.
///
/// Use as `schema: Some(schema_set::<MyType>)` in route metadata.
pub fn schema_set<T: utoipa::ToSchema>() -> NamedSchemaSet {
    let name = T::name().to_string();
    let mut components = Vec::new();
    T::schemas(&mut components);
    components.push((name.clone(), T::schema()));
    NamedSchemaSet {
        reference: RefOr::Ref(Ref::from_schema_name(name)),
        components,
    }
}

/// Build the documented query parameters of a [`utoipa::IntoParams`]
/// type.
///
/// Use as `query_params: Some(query_params::<MyParams>)` in route
/// metadata.
pub fn query_params<T: utoipa::IntoParams>() -> Vec<Parameter> {
    T::into_params(|| Some(ParameterIn::Query))
}
