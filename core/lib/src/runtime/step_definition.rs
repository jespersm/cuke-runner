//use std::time::Duration;
use std::any::TypeId;
use std::fmt;

use gherkin::cuke;

use api::CodeLocation;
use glue::step::{StaticStepDef, StepFn};
use glue::step::argument::{StepArgument, DocString, DataTable};
use runtime::Scenario;

use super::step_expression::StepExpression;

#[derive(Clone)]
pub struct StepDefinition {
    pub expression: StepExpression,
    pub parameter_infos: Vec<TypeId>,
//    pub timeout: Duration,
    pub step_fn: StepFn,
    pub location: CodeLocation,
}

impl fmt::Debug for StepDefinition {
    fn fmt(&self, f: &mut fmt::Formatter) -> ::std::result::Result<(), fmt::Error> {
        f.debug_struct("StepDefinition")
            .field("expression", &self.expression)
            .field("parameter_infos", &self.parameter_infos)
//            .field("timeout", &self.timeout)
            .field("step_fn", &"<step_fn>")
            .field("location", &self.location)
            .finish()
    }
}

impl From<&&StaticStepDef> for StepDefinition {
    fn from(static_step_def: &&StaticStepDef) -> Self {
        StepDefinition {
            expression: StepExpression::from_regex(static_step_def.expression),
            parameter_infos: Vec::new(),
            step_fn: static_step_def.step_fn,
            location: static_step_def.location,
        }
    }
}

impl StepDefinition {
    /// Returns the list of arguments for this step definition.
    ///
    /// Returns `None` if the step definition doesn't match at all.
    /// Returns an empty `Vec` if it matches with 0 arguments
    /// and bigger sizes if it matches several.
    pub fn matched_arguments<'s>(&'s self, step: &'s cuke::Step) -> Option<Vec<StepArgument<'s>>> {
        let mut matched_arguments = match self.expression.matched_arguments(&step.text) {
            Some(arguments) => arguments,
            None => return None,
        };

        match &step.argument {
            Some(argument) => {
                matched_arguments.reserve_exact(1);

                match argument {
                    cuke::Argument::String(ref string) =>
                        matched_arguments.push(StepArgument::DocString(DocString::from(string))),
                    cuke::Argument::Table(ref table) =>
                        matched_arguments.push(StepArgument::DataTable(DataTable::from(table))),
                }

                Some(matched_arguments)
            },
            None => Some(matched_arguments),
        }
    }

    /// The source line where the step definition is defined.
    ///
    /// Example: foo/bar/Zap.brainfuck:42
    pub fn get_location(&self) -> &CodeLocation {
        &self.location
    }

    /// The number of declared parameters of this step definition.
    pub fn get_parameter_count(&self) -> u8 {
        self.parameter_infos.len() as u8
    }

    /// Invokes the step definition.
    pub fn execute(&self, scenario: &mut Scenario, args: &[StepArgument])
        -> ::std::result::Result<(), ::glue::error::ExecutionError>
    {
        (self.step_fn)(&mut scenario.glue_scenario, args)
    }

    /// The step definition pattern for error reporting only.
    pub fn get_pattern(&self) -> &String {
        unimplemented!();
    }
}
