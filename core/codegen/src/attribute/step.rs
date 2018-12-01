use proc_macro::{Span, TokenStream};

use devise::{ext::TypeExt, FromMeta, Result, Spanned, SpanWrapped, syn};
use proc_macro2::TokenStream as TokenStream2;

use {PARAM_PREFIX, STEP_FN_PREFIX, STEP_STRUCT_PREFIX};
use glue_codegen::{StepKeyword, Regex};
use proc_macro_ext::{Diagnostics, StringLit};
use syn_ext::{IdentExt, syn_to_diag};

use self::syn::{Attribute, parse::Parser};

/// The raw, parsed `#[step]` attribute.
#[derive(Debug, FromMeta)]
struct StepAttribute {
    #[meta(naked)]
    keyword: SpanWrapped<StepKeyword>,
    expression: SpanWrapped<Regex>,
}

/// The raw, parsed `#[step]` (e.g, `given`, `when`, `then`) attribute.
#[derive(Debug, FromMeta)]
struct KeywordStepAttribute {
    #[meta(naked)]
    expression: SpanWrapped<Regex>,
}

/// This structure represents the parsed `step` attribute and associated items.
#[derive(Debug)]
struct Step {
    /// The status associated with the code in the `#[step(code)]` attribute.
    attribute: StepAttribute,
    /// The function that was decorated with the `step` attribute.
    function: syn::ItemFn,
    /// The parsed inputs to the user's function. The first ident is the ident
    /// as the user wrote it, while the second ident is the identifier that
    /// should be used during code generation, the `cuke_runner_ident`.
    inputs: Vec<(syn::Ident, syn::Ident, syn::Type)>,
}

fn parse_step(attr: StepAttribute, function: syn::ItemFn) -> Result<Step> {
    // Gather diagnostics as we proceed.
    let mut diags = Diagnostics::new();

    // Check the validity of function arguments.
    let mut inputs = vec![];
    for input in &function.decl.inputs {
        let help = "all handler arguments must be of the form: `ident: Type`";
        let span = input.span();
        let (ident, ty) = match input {
            syn::FnArg::Captured(arg) => match arg.pat {
                syn::Pat::Ident(ref pat) => (&pat.ident, &arg.ty),
                syn::Pat::Wild(_) => {
                    diags.push(span.error("handler arguments cannot be ignored").help(help));
                    continue;
                }
                _ => {
                    diags.push(span.error("invalid use of pattern").help(help));
                    continue;
                }
            }
            // Other cases shouldn't happen since we parsed an `ItemFn`.
            _ => {
                diags.push(span.error("invalid handler argument").help(help));
                continue;
            }
        };

        let cuke_runner_ident = ident.prepend(PARAM_PREFIX);
        inputs.push((ident.clone(), cuke_runner_ident, ty.with_stripped_lifetimes()));
    }

    diags.head_err_or(Step { attribute: attr, function, inputs })
}

fn step_data_expr(ident: &syn::Ident, ty: &syn::Type) -> TokenStream2 {
    let span = ident.span().unstable().join(ty.span()).unwrap().into();
    quote_spanned! { span =>
        #[allow(non_snake_case, unreachable_patterns)]
        let #ident: #ty = unimplemented!("step_data_expr");
    }
}

fn scenario_data_expr(ident: &syn::Ident, ty: &syn::Type) -> TokenStream2 {
    let span = ident.span().unstable().join(ty.span()).unwrap().into();
    quote_spanned! { span =>
        #[allow(non_snake_case, unreachable_patterns)]
        let #ident: #ty = match ::cuke_runner::glue::FromScenario::from_scenario(__scenario) {
            Ok(scenario_data) => scenario_data,
            Err(error) => {
                return Err(::cuke_runner::glue::ExecutionError::from(error))
            },
        };
    }
}

fn codegen_step(step: Step) -> Result<TokenStream> {

    // Gather everything we need.
    let (vis, user_handler_fn) = (&step.function.vis, &step.function);
    let user_handler_fn_name = &user_handler_fn.ident;
    let generated_fn_name = user_handler_fn_name.prepend(STEP_FN_PREFIX);
    let generated_struct_name = user_handler_fn_name.prepend(STEP_STRUCT_PREFIX);
    let parameter_names = step.inputs.iter().map(|(_, rocket_ident, _)| rocket_ident);
    let keyword = step.attribute.keyword;
    let expression = step.attribute.expression;

    let mut data_statements = Vec::with_capacity(step.inputs.len());
    // The first capture group is the entire regex which should not be considered
    let scenario_argument_count = expression.value.0.captures_len() - 1;
    for (index, (_ident, rocket_ident, ty)) in step.inputs.iter().enumerate() {
        if index < scenario_argument_count {
            data_statements.push(step_data_expr(rocket_ident, &ty));
        } else {
            data_statements.push(scenario_data_expr(rocket_ident, &ty));
        }
    }

    Ok(quote! {
        #user_handler_fn

        /// Cuke runner code generated wrapping step function.
        #vis fn #generated_fn_name(
            __scenario: &mut ::cuke_runner::glue::Scenario,
        ) -> ::std::result::Result<(), ::cuke_runner::glue::ExecutionError> {

            #(#data_statements)*

            // TODO: error handling...
            let result = ::std::panic::catch_unwind(::std::panic::AssertUnwindSafe(|| #user_handler_fn_name(#(#parameter_names),*)));
            match result {
                Ok(user_handler_fn_result) => return Ok(()),
                Err(err) => return Err(::cuke_runner::glue::panic_error(err)),
            };
        }

        /// Cuke runner code generated static step info.
        #[allow(non_upper_case_globals)]
        #vis static #generated_struct_name: ::cuke_runner::glue::StaticStepDefinition =
            ::cuke_runner::glue::StaticStepDefinition {
                name: stringify!(#user_handler_fn_name),
                keyword: #keyword,
                expression: #expression,
                step_fn: #generated_fn_name,
            };
    }.into())
}

fn complete_step(args: TokenStream2, input: TokenStream) -> Result<TokenStream> {
    let function: syn::ItemFn = syn::parse(input).map_err(syn_to_diag)
        .map_err(|diag| diag.help("`#[step]` can only be used on functions"))?;

    let full_attr = quote!(#[step(#args)]);
    let attrs = Attribute::parse_outer.parse2(full_attr).map_err(syn_to_diag)?;
    let attribute = match StepAttribute::from_attrs("step", &attrs) {
        Some(result) => result?,
        None => return Err(Span::call_site().error("internal error: bad attribute"))
    };

    codegen_step(parse_step(attribute, function)?)
}

fn incomplete_step(
    keyword: ::glue::StepKeyword,
    args: TokenStream2,
    input: TokenStream
) -> Result<TokenStream> {
    let keyword_str = keyword.to_string().to_lowercase();
    // FIXME(proc_macro): there should be a way to get this `Span`.
    let keyword_span = StringLit::new(format!("#[{}]", keyword), Span::call_site())
        .subspan(2..2 + keyword_str.len())
        .unwrap_or(Span::call_site());
    let keyword_ident = syn::Ident::new(&keyword_str, keyword_span.into());

    let function: syn::ItemFn = syn::parse(input).map_err(syn_to_diag)
        .map_err(|d| d.help(format!("#[{}] can only be used on functions", keyword_str)))?;

    let full_attr = quote!(#[#keyword_ident(#args)]);
    let attrs = Attribute::parse_outer.parse2(full_attr).map_err(syn_to_diag)?;
    let keyword_attribute = match KeywordStepAttribute::from_attrs(&keyword_str, &attrs) {
        Some(result) => result?,
        None => return Err(Span::call_site().error("internal error: bad attribute"))
    };

    let attribute = StepAttribute {
        keyword: SpanWrapped {
            full_span: keyword_span, span: keyword_span, value: StepKeyword(keyword)
        },
        expression: keyword_attribute.expression,
    };

    codegen_step(parse_step(attribute, function)?)
}

pub fn step_attribute<K: Into<Option<::glue::StepKeyword>>>(
    keyword: K,
    args: TokenStream,
    input: TokenStream
) -> TokenStream {
    let result = match keyword.into() {
        Some(keyword) => incomplete_step(keyword, args.into(), input),
        None => complete_step(args.into(), input)
    };

    result.unwrap_or_else(|diag| { diag.emit(); TokenStream::new() })
}
