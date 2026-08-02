use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Error, ItemFn, LitStr, Result, parse::Parse, parse::ParseStream, token::Comma};

pub(super) fn expand(args: TokenStream, function: ItemFn) -> Result<TokenStream> {
    let metadata: Metadata = syn::parse2(args)?;
    let function_name = function.sig.ident.clone();
    let constant_name = format_ident!("OPENAPI_ROUTE_{}", function_name.to_string().to_uppercase());
    let method = method_tokens(&metadata.method)?;
    let path = metadata.path;
    let operation_id = metadata
        .operation_id
        .unwrap_or_else(|| LitStr::new(&function_name.to_string(), function_name.span()));
    let (doc_summary, doc_description) = extract_docs(&function);
    let summary = metadata
        .summary
        .or(doc_summary)
        .unwrap_or_else(|| LitStr::new(&function_name.to_string(), function_name.span()));
    let description = metadata
        .description
        .or(doc_description)
        .map_or_else(|| quote! { None }, |value| quote! { Some(#value) });
    let tags = metadata.tags;
    let parameters = metadata.parameters;
    let request_type = metadata
        .request_type
        .or_else(|| infer_json_type(&function))
        .map_or_else(|| quote! { None }, |value| quote! { Some(#value) });
    let response_type = metadata
        .response_type
        .or_else(|| infer_result_type(&function, 0))
        .map_or_else(|| quote! { None }, |value| quote! { Some(#value) });
    let inferred_error_type = infer_result_type(&function, 1);
    let error_types = metadata.error_types;
    let error_types = if error_types.is_empty() {
        inferred_error_type.into_iter().collect::<Vec<_>>()
    } else {
        error_types
    };
    let parameter_tokens = parameters.iter().map(|parameter| {
        let name = &parameter.name;
        let description = parameter
            .description
            .as_ref()
            .map_or_else(|| quote! { None }, |value| quote! { Some(#value) });
        quote! {
            ::openapi_route::RouteParameter {
                name: #name,
                description: #description,
            }
        }
    });

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
                request_type: #request_type,
                response_type: #response_type,
                error_types: &[#(#error_types),*],
            };
    })
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
            .map(|segment| segment.ident.to_string())
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

struct Metadata {
    method: String,
    path: LitStr,
    operation_id: Option<LitStr>,
    summary: Option<LitStr>,
    description: Option<LitStr>,
    tags: Vec<LitStr>,
    parameters: Vec<Parameter>,
    request_type: Option<LitStr>,
    response_type: Option<LitStr>,
    error_types: Vec<LitStr>,
}

struct Parameter {
    name: LitStr,
    description: Option<LitStr>,
}

impl Parse for Metadata {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut method = None;
        let mut path = None;
        let mut operation_id = None;
        let mut summary = None;
        let mut description = None;
        let mut tags = Vec::new();
        let mut parameters = Vec::new();
        let mut request_type = None;
        let mut response_type = None;
        let mut error_types = Vec::new();

        while !input.is_empty() {
            let key: syn::Ident = input.parse()?;
            input.parse::<syn::Token![=]>()?;
            let value: LitStr = input.parse()?;
            match key.to_string().as_str() {
                "method" => method = Some(value.value()),
                "path" => path = Some(value),
                "operation_id" => operation_id = Some(value),
                "summary" => summary = Some(value),
                "description" => description = Some(value),
                "tag" => tags.push(value),
                "parameter" => parameters.push(Parameter {
                    name: value,
                    description: None,
                }),
                "request_type" => request_type = Some(value),
                "response_type" => response_type = Some(value),
                "error_type" => error_types.push(value),
                _ => return Err(Error::new(key.span(), "unknown openapi_handler option")),
            }
            if input.peek(Comma) {
                input.parse::<Comma>()?;
            }
        }

        Ok(Self {
            method: method
                .ok_or_else(|| Error::new(proc_macro2::Span::call_site(), "method is required"))?,
            path: path
                .ok_or_else(|| Error::new(proc_macro2::Span::call_site(), "path is required"))?,
            operation_id,
            summary,
            description,
            tags,
            parameters,
            request_type,
            response_type,
            error_types,
        })
    }
}
