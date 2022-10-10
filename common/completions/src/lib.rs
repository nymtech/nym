use clap::builder::Command;
use clap::clap_derive::ArgEnum;
use clap::Args;
use clap_complete::generator::generate;
use clap_complete::Shell as ClapShell;
use std::io;

pub fn fig_generate(command: &mut Command, name: &str) {
    clap_complete::generate(
        clap_complete_fig::Fig,
        command,
        name,
        &mut std::io::stdout(),
    )
}

#[derive(ArgEnum, Copy, Clone)]
pub enum Shell {
    Bash,
    Elvish,
    Fish,
    PowerShell,
    Zsh,
}

#[derive(Args, Copy, Clone)]
pub struct ArgShell {
    #[clap(arg_enum, value_name = "SHELL")]
    shell: Shell,
}

impl ArgShell {
    pub fn generate(&self, command: &mut Command, name: &str) {
        self.shell.generate(command, name)
    }
}

impl Shell {
    pub fn generate(&self, command: &mut Command, name: &str) {
        match &self {
            Self::Bash => {
                generate(ClapShell::Bash, command, name, &mut io::stdout());
            }
            Self::Elvish => {
                generate(ClapShell::Elvish, command, name, &mut io::stdout());
            }
            Self::Fish => {
                generate(ClapShell::Fish, command, name, &mut io::stdout());
            }
            Self::PowerShell => {
                generate(ClapShell::PowerShell, command, name, &mut io::stdout());
            }
            Self::Zsh => {
                generate(ClapShell::Zsh, command, name, &mut io::stdout());
            }
        }
    }
}
