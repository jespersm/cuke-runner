use quote::ToTokens;
use proc_macro2::TokenStream as TokenStream2;
use devise::{FromMeta, MetaItem, Result, ext::Split2};
use glue;

use proc_macro_ext::{SpanExt, StringLit};

#[derive(Debug)]
crate struct StepKeyword(crate glue::StepKeyword);

#[derive(Debug)]
crate struct Regex(crate regex::Regex);

#[derive(Clone, Debug)]
crate struct Optional<T>(crate Option<T>);

impl FromMeta for StringLit {
    fn from_meta(meta: MetaItem) -> Result<Self> {
        Ok(StringLit::new(String::from_meta(meta)?, meta.value_span()))
    }
}

const VALID_STEPS_STR: &str = "`Given`, `When`, `Then`";

const VALID_STEPS: &[glue::StepKeyword] = &[
    glue::StepKeyword::Given,
    glue::StepKeyword::When,
    glue::StepKeyword::Then,
    glue::StepKeyword::Star,
];

impl FromMeta for StepKeyword {
    fn from_meta(meta: MetaItem) -> Result<Self> {
        let span = meta.value_span();
        let help_text = format!("keyword must be one of: {}", VALID_STEPS_STR);

        if let MetaItem::Ident(ident) = meta {
            let keyword = ident.to_string().parse()
                .map_err(|_| span.error("invalid keyword").help(&*help_text))?;

            if !VALID_STEPS.contains(&keyword) {
                return Err(span.error("invalid keyword for step handlers").help(&*help_text));
            }

            return Ok(StepKeyword(keyword));
        }

        Err(span.error(format!("expected identifier, found {}", meta.description()))
                .help(&*help_text))
    }
}

impl ToTokens for StepKeyword {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let keyword_tokens = match self.0 {
            glue::StepKeyword::Star => quote!(::cuke_runner::glue::StepKeyword::Star),
            glue::StepKeyword::Given => quote!(::cuke_runner::glue::StepKeyword::Given),
            glue::StepKeyword::When => quote!(::cuke_runner::glue::StepKeyword::When),
            glue::StepKeyword::Then => quote!(::cuke_runner::glue::StepKeyword::Then),
        };

        tokens.extend(keyword_tokens);
    }
}

impl FromMeta for Regex {
    fn from_meta(meta: MetaItem) -> Result<Self> {
        let string = StringLit::from_meta(meta)?;
        let span = string.subspan(1..(string.len() + 1))
            .expect("regex");

        let result = regex::Regex::new(&string);
        match result {
            Ok(regex) => Ok(Regex(regex)),
            Err(err) => Err(span.error(format!("step expression \"{}\" is not a valid regex: {}", &*string, err))),
        }
    }
}

impl ToTokens for Regex {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let string = self.0.as_str();
        tokens.extend(quote!(#string));
    }
}

impl<T: ToTokens> ToTokens for Optional<T> {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let opt_tokens = match self.0 {
            Some(ref val) => quote!(Some(#val)),
            None => quote!(None)
        };

        tokens.extend(opt_tokens);
    }
}