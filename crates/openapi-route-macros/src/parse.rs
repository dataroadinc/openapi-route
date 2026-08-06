//! Attribute parsing for `#[openapi_handler(...)]`.

use syn::parse::{Parse, ParseStream};
use syn::token::Comma;
use syn::{Error, LitBool, LitInt, LitStr, Result};

/// The parsed macro attribute surface.
pub(crate) struct Metadata {
    pub(crate) service: Option<syn::Path>,
    pub(crate) method: String,
    pub(crate) path: LitStr,
    pub(crate) operation_id: Option<LitStr>,
    pub(crate) summary: Option<LitStr>,
    pub(crate) description: Option<LitStr>,
    pub(crate) tags: Vec<LitStr>,
    pub(crate) parameters: Vec<ParameterSpec>,
    pub(crate) query_params: Option<syn::Path>,
    pub(crate) request_body: Option<syn::Path>,
    pub(crate) request_content: Vec<LitStr>,
    pub(crate) request_type: Option<LitStr>,
    pub(crate) response_type: Option<LitStr>,
    pub(crate) error_types: Vec<LitStr>,
    pub(crate) responses: Vec<ResponseAttr>,
}

/// One `param(...)` group or legacy `parameter = "name"` entry.
pub(crate) struct ParameterSpec {
    pub(crate) name: LitStr,
    pub(crate) description: Option<LitStr>,
    pub(crate) location: Option<LitStr>,
    pub(crate) required: Option<LitBool>,
    pub(crate) schema: Option<syn::Path>,
}

/// One `response(...)` or `error(...)` group.
pub(crate) struct ResponseAttr {
    pub(crate) status: LitInt,
    pub(crate) body: Option<syn::Path>,
    pub(crate) content: Option<LitStr>,
    pub(crate) description: Option<LitStr>,
    pub(crate) example: Option<LitStr>,
    pub(crate) is_error: bool,
}

impl Parse for Metadata {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut metadata = Self {
            service: None,
            method: String::new(),
            path: LitStr::new("", proc_macro2::Span::call_site()),
            operation_id: None,
            summary: None,
            description: None,
            tags: Vec::new(),
            parameters: Vec::new(),
            query_params: None,
            request_body: None,
            request_content: Vec::new(),
            request_type: None,
            response_type: None,
            error_types: Vec::new(),
            responses: Vec::new(),
        };
        let mut method = None;
        let mut path = None;

        while !input.is_empty() {
            let key: syn::Ident = input.parse()?;
            let key_name = key.to_string();
            if input.peek(syn::token::Paren) {
                let group;
                syn::parenthesized!(group in input);
                match key_name.as_str() {
                    "param" => metadata.parameters.push(parse_param_group(&group)?),
                    "response" => metadata
                        .responses
                        .push(parse_response_group(&group, false)?),
                    "error" => metadata.responses.push(parse_response_group(&group, true)?),
                    _ => return Err(Error::new(key.span(), "unknown openapi_handler group")),
                }
            } else {
                input.parse::<syn::Token![=]>()?;
                match key_name.as_str() {
                    "service" => metadata.service = Some(input.parse()?),
                    "query_params" => metadata.query_params = Some(input.parse()?),
                    "request_body" => metadata.request_body = Some(input.parse()?),
                    "method" => method = Some(input.parse::<LitStr>()?.value()),
                    "path" => path = Some(input.parse()?),
                    "operation_id" => metadata.operation_id = Some(input.parse()?),
                    "summary" => metadata.summary = Some(input.parse()?),
                    "description" => metadata.description = Some(input.parse()?),
                    "tag" => metadata.tags.push(input.parse()?),
                    "parameter" => metadata.parameters.push(ParameterSpec {
                        name: input.parse()?,
                        description: None,
                        location: None,
                        required: None,
                        schema: None,
                    }),
                    "request_content" => metadata.request_content.push(input.parse()?),
                    "request_type" => metadata.request_type = Some(input.parse()?),
                    "response_type" => metadata.response_type = Some(input.parse()?),
                    "error_type" => metadata.error_types.push(input.parse()?),
                    _ => return Err(Error::new(key.span(), "unknown openapi_handler option")),
                }
            }
            if input.peek(Comma) {
                input.parse::<Comma>()?;
            }
        }

        metadata.method = method
            .ok_or_else(|| Error::new(proc_macro2::Span::call_site(), "method is required"))?;
        metadata.path =
            path.ok_or_else(|| Error::new(proc_macro2::Span::call_site(), "path is required"))?;
        Ok(metadata)
    }
}

fn parse_param_group(input: ParseStream<'_>) -> Result<ParameterSpec> {
    let mut name = None;
    let mut description = None;
    let mut location = None;
    let mut required = None;
    let mut schema = None;
    while !input.is_empty() {
        let key: syn::Ident = input.parse()?;
        input.parse::<syn::Token![=]>()?;
        match key.to_string().as_str() {
            "name" => name = Some(input.parse()?),
            "description" => description = Some(input.parse()?),
            "location" => location = Some(input.parse()?),
            "required" => required = Some(input.parse()?),
            "schema" => schema = Some(input.parse()?),
            _ => return Err(Error::new(key.span(), "unknown param option")),
        }
        if input.peek(Comma) {
            input.parse::<Comma>()?;
        }
    }
    Ok(ParameterSpec {
        name: name.ok_or_else(|| {
            Error::new(
                proc_macro2::Span::call_site(),
                "param requires name = \"...\"",
            )
        })?,
        description,
        location,
        required,
        schema,
    })
}

fn parse_response_group(input: ParseStream<'_>, is_error: bool) -> Result<ResponseAttr> {
    let mut status = None;
    let mut body = None;
    let mut content = None;
    let mut description = None;
    let mut example = None;
    while !input.is_empty() {
        let key: syn::Ident = input.parse()?;
        input.parse::<syn::Token![=]>()?;
        match key.to_string().as_str() {
            "status" => status = Some(input.parse()?),
            "body" => body = Some(input.parse()?),
            "content" => content = Some(input.parse()?),
            "description" => description = Some(input.parse()?),
            "example" => example = Some(input.parse()?),
            _ => return Err(Error::new(key.span(), "unknown response option")),
        }
        if input.peek(Comma) {
            input.parse::<Comma>()?;
        }
    }
    Ok(ResponseAttr {
        status: status.ok_or_else(|| {
            Error::new(
                proc_macro2::Span::call_site(),
                "response requires status = <code>",
            )
        })?,
        body,
        content,
        description,
        example,
        is_error,
    })
}
