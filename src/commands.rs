use std::{collections::HashMap, fmt::Display};

use anyhow::Result;

#[derive(Debug)]
pub enum ParseError {
    UnknownCommand {
        path: Vec<&'static str>,
        name: String,
    },
    UnknownArg {
        path: Vec<&'static str>,
        name: String,
    },
    MissingArgValue {
        path: Vec<&'static str>,
        name: String,
    },
    UnexpectedArgValue {
        path: Vec<&'static str>,
        name: String,
    },
    MissingValue {
        path: Vec<&'static str>,
        name: String,
    },
    HelpRequested {
        path: Vec<&'static str>,
    },
}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::UnknownCommand { path, name } => {
                write!(f, "Unknown command '{}' at '{}'", name, path.join(" "))
            }
            ParseError::UnknownArg { path, name } => {
                write!(f, "Unknown argument '{}' at '{}'", name, path.join(" "))
            }
            ParseError::MissingArgValue { path, name } => {
                write!(
                    f,
                    "Missing value for argument '{}' at '{}'",
                    name,
                    path.join(" ")
                )
            }
            ParseError::UnexpectedArgValue { path, name } => {
                write!(
                    f,
                    "Unexpected value for flag argument '{}' at '{}'",
                    name,
                    path.join(" ")
                )
            }
            ParseError::MissingValue { path, name } => {
                write!(
                    f,
                    "Missing value for command '{}' at '{}'",
                    name,
                    path.join(" ")
                )
            }
            ParseError::HelpRequested { path } => {
                write!(f, "Help requested at '{}'", path.join(" "))
            }
        }
    }
}

#[derive(Debug)]
pub struct CommandDef {
    name: &'static str,
    description: &'static str,
    args: Vec<ArgDef>,
    subcommands: Vec<CommandDef>,
}

#[derive(Debug, PartialEq)]
pub enum ArgType {
    Flag,
    Value,
}

#[derive(Debug)]
pub struct ArgDef {
    short: &'static str,
    long: &'static str,
    description: &'static str,
    arg_type: ArgType,
}

impl CommandDef {
    pub fn new(name: &'static str, description: &'static str) -> Self {
        CommandDef {
            name: name,
            description: description,
            args: vec![],
            subcommands: vec![],
        }
    }

    pub fn add_arg(
        mut self,
        short: &'static str,
        long: &'static str,
        arg_type: ArgType,
        description: &'static str,
    ) -> Self {
        let arg = ArgDef {
            short: short,
            long: long,
            description: description,
            arg_type,
        };

        self.args.push(arg);

        return self;
    }

    pub fn add_subcommand(mut self, subcommand: CommandDef) -> Self {
        self.subcommands.push(subcommand);
        self
    }

    pub fn get_help(&self, path: &[&str]) -> String {
        let Some(command) = self.get_command_from_path(path) else {
            return format!("Unknown command: {}", path.join(" "));
        };

        let mut help_text = format!("Command: {}\n{}\n", command.name, command.description);
        help_text.push_str("Arguments:\n");
        for arg in &command.args {
            help_text.push_str(&format!(
                "  -{}, --{}: {} ({})\n",
                arg.short,
                arg.long,
                arg.description,
                match arg.arg_type {
                    ArgType::Flag => "flag",
                    ArgType::Value => "value",
                }
            ));
        }

        help_text.push_str("\nSubcommands:\n");
        for sub in &command.subcommands {
            help_text.push_str(&format!("  {}: {}\n", sub.name, sub.description));
        }

        help_text
    }

    fn get_command_from_path(&self, path: &[&str]) -> Option<&CommandDef> {
        let len = path.len();
        if len == 0 {
            return None;
        }

        if len == 1 && self.name == path[0] {
            return Some(self);
        }

        if len > 1 {
            if let Some(command) = self.find_subcommand(&path[1]) {
                return command.get_command_from_path(&path[1..]);
            }
        }

        None
    }

    pub fn parse(&self, args: std::env::Args) -> Result<Command, ParseError> {
        let args_vec: Vec<String> = args.collect();
        let tokens = tokenize(&args_vec);
        println!("tokens: {:?}", tokens);
        self.parse_intrnal(&tokens, vec![])
    }

    fn parse_intrnal(
        &self,
        tokens: &[Token],
        mut path: Vec<&'static str>,
    ) -> Result<Command, ParseError> {
        let mut args = HashMap::new();
        let mut positional = vec![];
        let mut can_be_subcommand = self.subcommands.len() > 0;

        path.push(self.name);

        let mut i = 1;
        let mut everything_is_positional = false;

        while i < tokens.len() {
            let arg = &tokens[i];

            if everything_is_positional {
                if let Token::Word(word) = arg {
                    positional.push(word.to_string());
                }
                i += 1;
                continue;
            }

            if let Token::EndOfOptions = arg {
                everything_is_positional = true;
                i += 1;
                continue;
            }

            if let Token::Word(word) = arg
                && word == "help"
            {
                return Err(ParseError::HelpRequested { path });
            }

            if can_be_subcommand && let Token::Word(name) = arg {
                let sub_def = self.find_subcommand(name);
                if let Some(def) = sub_def {
                    let command = def.parse_intrnal(&tokens[i..], path)?;
                    return Ok(command);
                }
            }

            can_be_subcommand = false;

            if let Token::Long(name) | Token::Short(name) = arg {
                if name == "help" || name == "h" {
                    return Err(ParseError::HelpRequested { path });
                }
                let Some(arg_def) = self.find_arg(name) else {
                    return Err(ParseError::UnknownArg {
                        path,
                        name: name.to_string(),
                    });
                };

                let parsed_arg = match arg_def.arg_type {
                    ArgType::Flag => Arg::Flag,
                    ArgType::Value => {
                        let val_i = i + 1;
                        if val_i >= tokens.len() {
                            return Err(ParseError::MissingArgValue {
                                path,
                                name: name.to_string(),
                            });
                        }

                        let next = &tokens[i + 1];

                        let Token::Word(val) = next else {
                            return Err(ParseError::MissingArgValue {
                                path,
                                name: name.to_string(),
                            });
                        };

                        i += 1;
                        Arg::Value(val.to_string())
                    }
                };

                args.insert(arg_def.long, parsed_arg);
                i += 1;

                continue;
            }

            if let Token::LongWithValue(name, val) = arg {
                let Some(arg_def) = self.find_arg(&name) else {
                    return Err(ParseError::UnknownArg {
                        path,
                        name: name.to_string(),
                    });
                };

                if arg_def.arg_type == ArgType::Flag {
                    return Err(ParseError::UnexpectedArgValue {
                        path,
                        name: name.to_string(),
                    });
                }

                let parsed_arg = Arg::Value(val.to_string());

                args.insert(arg_def.long, parsed_arg);
            }

            if let Token::Word(word) = arg {
                positional.push(word.to_string());
            }

            i += 1;
        }

        Ok(Command {
            path,
            args,
            positional,
        })
    }

    fn find_subcommand(&self, name: &str) -> Option<&CommandDef> {
        self.subcommands.iter().find(|s| s.name == name)
    }

    fn find_arg(&self, name: &str) -> Option<&ArgDef> {
        self.args.iter().find(|s| s.long == name || s.short == name)
    }
}

#[derive(Debug)]
enum Token {
    Short(String),
    Long(String),
    LongWithValue(String, String),
    EndOfOptions,
    Word(String),
}

fn tokenize(args: &[String]) -> Vec<Token> {
    let mut out = vec![];
    let mut everything_is_positional = false;
    for arg in args {
        if everything_is_positional {
            out.push(Token::Word(arg.to_string()));
            continue;
        }

        if arg == "--" {
            out.push(Token::EndOfOptions);
            everything_is_positional = true;
            continue;
        }

        if let Some(rest) = arg.strip_prefix("--") {
            if let Some((k, v)) = rest.split_once("=") {
                out.push(Token::LongWithValue(k.to_string(), v.to_string()));
            } else {
                out.push(Token::Long(rest.to_string()));
            }
            continue;
        }

        if let Some(rest) = arg.strip_prefix("-") {
            out.push(Token::Short(rest.to_string()));
            continue;
        }

        out.push(Token::Word(arg.to_string()))
    }

    out
}

#[derive(Debug)]
pub struct Command {
    path: Vec<&'static str>,
    args: HashMap<&'static str, Arg>,
    positional: Vec<String>,
}

#[derive(Debug)]
pub enum Arg {
    Value(String),
    Flag,
}

impl Command {
    pub fn get_path(&self) -> &[&'static str] {
        &self.path
    }

    pub fn get_arg(&self, long: &str) -> Option<&Arg> {
        self.args.get(long)
    }

    pub fn get_positional_string(&self) -> String {
        self.positional.join(" ")
    }
}
