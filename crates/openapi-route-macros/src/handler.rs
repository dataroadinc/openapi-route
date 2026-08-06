//! Expansion of `#[openapi_handler(...)]` into route metadata statics.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Error, ItemFn, LitStr, Result};

use crate::parse::{Metadata, ParameterSpec, ResponseAttr};

pub(super) fn expand(args: TokenStream, function: ItemFn) -> Result<TokenStream> {
    let metadata: Metadata = syn::parse2(args)?;
    let service = metadata
        .service
        .clone()
        .unwrap_or_else(|| syn::parse_quote!(crate::API_SERVICE));
    let function_name = function.sig.ident.clone();
    let constant_name = format_ident!("OPENAPI_ROUTE_{}", function_name.to_string().to_uppercase());
    let method = method_tokens(&metadata.method)?;
    let path = &metadata.path;
    let operation_id = metadata
        .operation_id
        .clone()
        .unwrap_or_else(|| LitStr::new(&function_name.to_string(), function_name.span()));
    let (doc_summary, doc_description) = extract_docs(&function);
    let summary = metadata
        .summary
        .clone()
        .or(doc_summary)
        .unwrap_or_else(|| LitStr::new(&function_name.to_string(), function_name.span()));
    let description = metadata
        .description
        .clone()
        .or(doc_description)
        .map_or_else(|| quote! { None }, |value| quote! { Some(#value) });
    let tags = &metadata.tags;
    let parameter_tokens = metadata
        .parameters
        .iter()
        .map(parameter_tokens)
        .collect::<Result<Vec<_>>>()?;
    let query_params = metadata.query_params.as_ref().map_or_else(
        || quote! { None },
        |ty| quote! { Some(::openapi_route::query_params::<#ty>) },
    );
    let request = request_tokens(&metadata, &function);
    let responses = response_tokens(&metadata, &function);

    Ok(quote! {
        #function

        #[doc(hidden)]
        pub static #constant_name: ::openapi_route::RouteMetadata =
            ::openapi_route::RouteMetadata {
                method: #method,
                path: #path,
                operation_id: #operation_id,
                summary: #summary,
                description: #description,
                tags: &[#(#tags),*],
                parameters: &[#(#parameter_tokens),*],
                query_params: #query_params,
                request: #request,
                responses: &[#(#responses),*],
            };

        ::openapi_route::inventory::submit! {
            ::openapi_route::RegisteredRoute {
                service: &#service,
                route: &#constant_name,
            }
        }
    })
}

fn parameter_tokens(parameter: &ParameterSpec) -> Result<TokenStream> {
    let name = &parameter.name;
    let description = parameter
        .description
        .as_ref()
        .map_or_else(|| quote! { None }, |value| quote! { Some(#value) });
    let location = match parameter.location.as_ref().map(|value| value.value()) {
        None => quote! { ::openapi_route::ParameterLocation::Path },
        Some(value) => match value.as_str() {
            "path" => quote! { ::openapi_route::ParameterLocation::Path },
            "query" => quote! { ::openapi_route::ParameterLocation::Query },
            "header" => quote! { ::openapi_route::ParameterLocation::Header },
            _ => {
                return Err(Error::new(
                    parameter
                        .location
                        .as_ref()
                        .expect("location present")
                        .span(),
                    "location must be \"path\", \"query\", or \"header\"",
                ));
            }
        },
    };
    let required = parameter.required.as_ref().map_or_else(
        || {
            let default_required = parameter
                .location
                .as_ref()
                .is_none_or(|value| value.value() == "path");
            quote! { #default_required }
        },
        |value| quote! { #value },
    );
    let schema = parameter.schema.as_ref().map_or_else(
        || quote! { None },
        |ty| quote! { Some(::openapi_route::schema_set::<#ty>) },
    );
    Ok(quote! {
        ::openapi_route::RouteParameter {
            name: #name,
            description: #description,
            location: #location,
            required: #required,
            schema: #schema,
        }
    })
}

fn request_tokens(metadata: &Metadata, function: &ItemFn) -> TokenStream {
    if let Some(ty) = &metadata.request_body {
        let type_name = last_segment_name(ty);
        let media_type = metadata
            .request_content
            .first()
            .map_or_else(|| quote! { "application/json" }, |value| quote! { #value });
        return quote! {
            Some(::openapi_route::RequestSpec {
                type_name: Some(#type_name),
                required: true,
                contents: &[::openapi_route::ContentSpec {
                    media_type: #media_type,
                    schema: Some(::openapi_route::schema_set::<#ty>),
                    example: None,
                }],
            })
        };
    }
    let prose = metadata
        .request_type
        .clone()
        .or_else(|| infer_json_type(function));
    if !metadata.request_content.is_empty() {
        // Raw (non-Serde) request representations: one prose content
        // entry per declared media type, no schema.
        let type_name = prose
            .as_ref()
            .map_or_else(|| quote! { None }, |value| quote! { Some(#value) });
        let contents = metadata.request_content.iter().map(|media_type| {
            quote! {
                ::openapi_route::ContentSpec {
                    media_type: #media_type,
                    schema: None,
                    example: None,
                }
            }
        });
        return quote! {
            Some(::openapi_route::RequestSpec {
                type_name: #type_name,
                required: true,
                contents: &[#(#contents),*],
            })
        };
    }
    match prose {
        Some(type_name) => quote! {
            Some(::openapi_route::RequestSpec {
                type_name: Some(#type_name),
                required: true,
                contents: &[::openapi_route::ContentSpec::PROSE_JSON],
            })
        },
        None => quote! { None },
    }
}

fn response_tokens(metadata: &Metadata, function: &ItemFn) -> Vec<TokenStream> {
    let mut responses = Vec::new();
    let mut has_success = false;
    let mut has_error = false;
    for attribute in &metadata.responses {
        if attribute.is_error {
            has_error = true;
        } else {
            has_success = true;
        }
        responses.push(response_attr_tokens(attribute));
    }

    if !has_success
        && let Some(type_name) = metadata
            .response_type
            .clone()
            .or_else(|| infer_result_type(function, 0))
    {
        let description = format!("Successful response containing {}.", type_name.value());
        responses.insert(
            0,
            quote! {
                ::openapi_route::ResponseSpec {
                    status: 200,
                    description: #description,
                    type_name: Some(#type_name),
                    contents: &[::openapi_route::ContentSpec::PROSE_JSON],
                }
            },
        );
    }

    let mut error_names = metadata
        .error_types
        .iter()
        .map(LitStr::value)
        .collect::<Vec<_>>();
    if error_names.is_empty()
        && let Some(inferred) = infer_result_type(function, 1)
    {
        error_names.push(inferred.value());
    }
    if !has_error && !error_names.is_empty() {
        let description = format!("Request failed with one of: {}.", error_names.join(", "));
        responses.push(quote! {
            ::openapi_route::ResponseSpec {
                status: 400,
                description: #description,
                type_name: None,
                contents: &[],
            }
        });
    }
    responses
}

fn response_attr_tokens(attribute: &ResponseAttr) -> TokenStream {
    let status = &attribute.status;
    let type_name_value = attribute.body.as_ref().map(last_segment_name);
    let description_value = attribute.description.as_ref().map_or_else(
        || default_response_description(attribute, type_name_value.as_deref()),
        |value| value.value(),
    );
    let type_name = type_name_value
        .as_ref()
        .map_or_else(|| quote! { None }, |value| quote! { Some(#value) });
    let media_type = attribute
        .content
        .as_ref()
        .map_or_else(|| quote! { "application/json" }, |value| quote! { #value });
    let example = attribute
        .example
        .as_ref()
        .map_or_else(|| quote! { None }, |value| quote! { Some(#value) });
    let contents = match (&attribute.body, &attribute.content, &attribute.example) {
        (None, None, None) => quote! { &[] },
        (Some(ty), _, _) => quote! {
            &[::openapi_route::ContentSpec {
                media_type: #media_type,
                schema: Some(::openapi_route::schema_set::<#ty>),
                example: #example,
            }]
        },
        (None, _, _) => quote! {
            &[::openapi_route::ContentSpec {
                media_type: #media_type,
                schema: None,
                example: #example,
            }]
        },
    };
    quote! {
        ::openapi_route::ResponseSpec {
            status: #status,
            description: #description_value,
            type_name: #type_name,
            contents: #contents,
        }
    }
}

fn default_response_description(attribute: &ResponseAttr, type_name: Option<&str>) -> String {
    match (attribute.is_error, type_name) {
        (false, Some(name)) => format!("Successful response containing {name}."),
        (false, None) => "Successful response".to_owned(),
        (true, Some(name)) => format!("Request failed with {name}."),
        (true, None) => "Error response".to_owned(),
    }
}

fn last_segment_name(path: &syn::Path) -> String {
    path.segments
        .last()
        .map_or_else(|| "Type".to_owned(), |segment| segment.ident.to_string())
}

fn extract_docs(function: &ItemFn) -> (Option<LitStr>, Option<LitStr>) {
    let lines = function
        .attrs
        .iter()
        .filter_map(|attribute| {
            if !attribute.path().is_ident("doc") {
                return None;
            }
            let syn::Meta::NameValue(meta) = &attribute.meta else {
                return None;
            };
            let syn::Expr::Lit(expression) = &meta.value else {
                return None;
            };
            let syn::Lit::Str(value) = &expression.lit else {
                return None;
            };
            let line = value.value().trim().to_owned();
            (!line.is_empty()).then_some(line)
        })
        .collect::<Vec<_>>();

    let Some(summary) = lines.first() else {
        return (None, None);
    };
    let summary = LitStr::new(&normalize_summary(summary), function.sig.ident.span());
    let description = if lines.len() > 1 {
        Some(LitStr::new(
            &lines[1..].join("\n"),
            function.sig.ident.span(),
        ))
    } else {
        None
    };
    (Some(summary), description)
}

fn normalize_summary(line: &str) -> String {
    for separator in ['—', '–'] {
        if let Some((_, summary)) = line.split_once(separator) {
            return summary.trim().to_owned();
        }
    }
    if let Some((_, summary)) = line.split_once("--") {
        return summary.trim().to_owned();
    }
    line.trim_matches('`').trim().to_owned()
}

fn infer_json_type(function: &ItemFn) -> Option<LitStr> {
    function.sig.inputs.iter().find_map(|argument| {
        let syn::FnArg::Typed(argument) = argument else {
            return None;
        };
        let syn::Type::Path(path) = argument.ty.as_ref() else {
            return None;
        };
        let segment = path.path.segments.last()?;
        if segment.ident != "Json" {
            return None;
        }
        let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments else {
            return None;
        };
        let syn::GenericArgument::Type(inner) = arguments.args.first()? else {
            return None;
        };
        Some(LitStr::new(&type_name(inner), segment.ident.span()))
    })
}

fn infer_result_type(function: &ItemFn, index: usize) -> Option<LitStr> {
    let syn::ReturnType::Type(_, output) = &function.sig.output else {
        return None;
    };
    let syn::Type::Path(path) = output.as_ref() else {
        return None;
    };
    let segment = path.path.segments.last()?;
    if segment.ident != "Result" {
        return None;
    }
    let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return None;
    };
    let syn::GenericArgument::Type(inner) = arguments.args.get(index)? else {
        return None;
    };
    Some(LitStr::new(&type_name(inner), segment.ident.span()))
}

fn type_name(ty: &syn::Type) -> String {
    match ty {
        syn::Type::Path(path) => path
            .path
            .segments
            .iter()
            .map(|segment| {
                let arguments = match &segment.arguments {
                    syn::PathArguments::AngleBracketed(arguments) => {
                        let types = arguments
                            .args
                            .iter()
                            .filter_map(|argument| match argument {
                                syn::GenericArgument::Type(ty) => Some(type_name(ty)),
                                _ => None,
                            })
                            .collect::<Vec<_>>();
                        if types.is_empty() {
                            String::new()
                        } else {
                            format!("<{}>", types.join(", "))
                        }
                    }
                    _ => String::new(),
                };
                format!("{}{}", segment.ident, arguments)
            })
            .collect::<Vec<_>>()
            .join("::"),
        _ => "Response".to_owned(),
    }
}

fn method_tokens(method: &str) -> Result<TokenStream> {
    let tokens = match method {
        "GET" => quote! { ::openapi_route::Method::Get },
        "POST" => quote! { ::openapi_route::Method::Post },
        "PUT" => quote! { ::openapi_route::Method::Put },
        "PATCH" => quote! { ::openapi_route::Method::Patch },
        "DELETE" => quote! { ::openapi_route::Method::Delete },
        "HEAD" => quote! { ::openapi_route::Method::Head },
        _ => {
            return Err(Error::new(
                proc_macro2::Span::call_site(),
                "unsupported HTTP method",
            ));
        }
    };
    Ok(tokens)
}
