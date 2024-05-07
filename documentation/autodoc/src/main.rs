use log::info;
use std::fs::File;
use std::io::{self, Write};
use std::process::{Command, Output};

fn main() -> io::Result<()> {
    env_logger::init();

    let commands_with_subcommands = vec![
        (
            "../../target/release/nym-client",
            vec![
                "init",
                "run",
                "import-credential",
                "list-gateways",
                "switch-gateway",
                "build-info",
                "completions",
                "generate-fig-spec",
            ],
        ),
        (
            "../../target/release/nym-node",
            vec![
                "build-info",
                "bonding-information",
                "node-details",
                "migrate",
                "run",
                "sign",
            ],
        ),
    ];

    for (main_command, subcommands) in commands_with_subcommands {
        let last_word = get_last_word_from_filepath(main_command);
        info!("{last_word:#?}");
        let mut file = File::create(format!("{}.md", last_word.unwrap()))?;
        writeln!(file, "# Command Output")?;
        writeln!(file, "\n## {}", main_command)?;
        for subcommand in subcommands {
            execute_command(&mut file, main_command, subcommand)?;
        }
    }
    Ok(())
}

fn get_last_word_from_filepath(filepath: &str) -> Option<&str> {
    let parts: Vec<&str> = filepath.split('/').collect();
    parts.last().copied()
}

fn execute_command(file: &mut File, main_command: &str, subcommand: &str) -> io::Result<()> {
    // first execute the command with `--help`
    info!("executing {} {} --help ", main_command, subcommand);
    let output = Command::new(main_command)
        .arg(subcommand)
        .arg("--help")
        .output()?;
    write_output_to_file(file, subcommand, output)?;

    // this check is basically checking for the rare commands (rn just one) that start a process with no params
    // perhaps if this list grows we could just add a timeout and shunt the running and writing
    // into a thread with a timeout or something but for right now its fine / thats overkill
    if get_last_word_from_filepath(main_command).unwrap() == "nym-node" && subcommand == "run" {
        info!("SKIPPING {} {}", main_command, subcommand);
    } else {
        info!("executing {} {}", main_command, subcommand);
        let output = Command::new(main_command).arg(subcommand).output()?;
        write_output_to_file(file, subcommand, output)?;
    }
    Ok(())
}

fn write_output_to_file(file: &mut File, subcommand: &str, output: Output) -> io::Result<()> {
    writeln!(file, "\n### {}", subcommand)?;

    // right now this is good to do imo as it shows us whats failing and whats not
    // in 'prod' we don't need to write "stdout"
    // see comment below re: keeping this in
    writeln!(file, "#### stdout\n```")?;
    file.write_all(&output.stdout)?;
    writeln!(file, "```")?;

    // if we want to keep this we could create 2 copies if you run this in a certain mode ("debug"
    // oder so) so you can see the stderr but in reality, we want dont want the errors in teh md
    // file and also probaly not write 'stdout', but just have <command><output> in a clean fashion
    writeln!(file, "#### stderr\n```")?;
    file.write_all(&output.stderr)?;
    writeln!(file, "```")?;

    Ok(())
}
