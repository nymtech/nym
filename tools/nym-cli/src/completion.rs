use clap::Command;
use clap_complete::generate;
use clap_complete_fig::Fig;

use std::io;

/// Generates a file with a Typescript export of type Fig.Spec (use https://www.npmjs.com/package/@withfig/autocomplete-tools for typings)
pub(crate) fn print_fig(cmd: &mut Command) {
    generate(Fig, cmd, cmd.get_name().to_string(), &mut io::stdout());
}
