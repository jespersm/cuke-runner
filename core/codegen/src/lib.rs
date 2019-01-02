#![feature(proc_macro_diagnostic)]
#![feature(proc_macro_span)]
#![feature(proc_macro_def_site)]
#![feature(crate_visibility_modifier)]
#![feature(rustc_private)]
#![recursion_limit="128"]

extern crate cuke_runner_glue as glue;
extern crate proc_macro;
extern crate proc_macro2;
#[macro_use]
extern crate quote;
extern crate regex;
extern crate syn;
#[macro_use]
extern crate log;
extern crate devise;
extern crate syntax_pos;

use proc_macro2::Span;
use proc_macro::TokenStream;

use glue::{HookType, StepKeyword};

#[macro_use]
mod proc_macro_ext;
mod attribute;
mod bang;
mod glue_codegen;
mod syn_ext;

crate static BEFORE_SCENARIO_HOOK_STRUCT_PREFIX: &str = "static_cuke_runner_before_scenario_hook_info_for_";
crate static BEFORE_SCENARIO_HOOK_FN_PREFIX: &str = "cuke_runner_before_scenario_hook_fn_";
crate static BEFORE_STEP_HOOK_STRUCT_PREFIX: &str = "static_cuke_runner_before_scenario_hook_info_for_";
crate static BEFORE_STEP_HOOK_FN_PREFIX: &str = "cuke_runner_before_scenario_hook_fn_";
crate static AFTER_STEP_HOOK_STRUCT_PREFIX: &str = "static_cuke_runner_before_scenario_hook_info_for_";
crate static AFTER_STEP_HOOK_FN_PREFIX: &str = "cuke_runner_before_scenario_hook_fn_";
crate static AFTER_SCENARIO_HOOK_STRUCT_PREFIX: &str = "static_cuke_runner_before_scenario_hook_info_for_";
crate static AFTER_SCENARIO_HOOK_FN_PREFIX: &str = "cuke_runner_before_scenario_hook_fn_";

crate static STEP_STRUCT_PREFIX: &str = "static_cuke_runner_step_info_for_";
crate static STEP_FN_PREFIX: &str = "cuke_runner_step_fn_";

crate static PARAM_PREFIX: &str = "__cuke_runner_param_";

macro_rules! emit {
    ($tokens:expr) => ({
        let tokens = $tokens;
        if ::std::env::var_os("CUKE_CODEGEN_DEBUG").is_some() {
            ::proc_macro::Span::call_site()
                .note("emitting cuke runner code generation debug output")
                .note(tokens.to_string())
                .emit()
        }

        tokens
    })
}

macro_rules! hook_attribute {
    ($name:ident => $hook_type:expr) => (
        #[proc_macro_attribute]
        pub fn $name(args: TokenStream, input: TokenStream) -> TokenStream {
            emit!(attribute::hook::hook_attribute($hook_type, args, input))
        }
    )
}

hook_attribute!(hook => None);
hook_attribute!(before_scenario => HookType::BeforeScenario);
hook_attribute!(before_step => HookType::BeforeStep);
hook_attribute!(after_step => HookType::AfterStep);
hook_attribute!(after_scenario => HookType::AfterScenario);

macro_rules! step_attribute {
    ($name:ident => $keyword:expr) => (
        #[proc_macro_attribute]
        pub fn $name(args: TokenStream, input: TokenStream) -> TokenStream {
            emit!(attribute::step::step_attribute($keyword, args, input))
        }
    )
}

step_attribute!(step => StepKeyword::Star);
step_attribute!(given => StepKeyword::Given);
step_attribute!(when => StepKeyword::When);
step_attribute!(then => StepKeyword::Then);


#[proc_macro]
pub fn generate_glue(input: TokenStream) -> TokenStream {
    emit!(bang::generate_glue_macro(input))
}

#[proc_macro]
pub fn glue(_input: TokenStream) -> TokenStream {
    // TODO: Use input instead of hardcoded glue module
    let call_site_span = Span::call_site();
    let glue = quote_spanned! {call_site_span=>
        Glue::from(::cuke_runner::glue::StaticGlueDefinitions {
            before_scenario_hooks: glue::BEFORE_SCENARIO_HOOK_DEFINITIONS,
            before_step_hooks: glue::BEFORE_STEP_HOOK_DEFINITIONS,
            steps: glue::STEP_DEFINITIONS,
            after_step_hooks: glue::AFTER_STEP_HOOK_DEFINITIONS,
            after_scenario_hooks: glue::AFTER_SCENARIO_HOOK_DEFINITIONS,
        })
    };

    TokenStream::from(glue)
}
