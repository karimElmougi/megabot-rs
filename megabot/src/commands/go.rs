use serenity::builder::CreateApplicationCommand;
use serenity::model::prelude::command::CommandOptionType;
use serenity::model::prelude::interaction::application_command::{
    CommandDataOption, CommandDataOptionValue,
};

pub fn run(options: &[CommandDataOption], link_store: &kv::Store<String>) -> String {
    let shortcut = get_shortcut(options).unwrap();
    match get_link(options) {
        Some(link) => match link_store.set(&shortcut, &link) {
            Ok(()) => format!("{link} was registered under {shortcut}!"),
            Err(e) => {
                log::error!("Link store error: {e}");
                format!("Server Error: Unable to register link :(")
            }
        },
        None => match link_store.get(&shortcut) {
            Ok(Some(link)) => link,
            Ok(None) => format!("No link registered under `{shortcut}`"),
            Err(e) => {
                log::error!("Link store error: {e}");
                format!("Server Error: Unable to fetch link :(")
            }
        },
    }
}

fn get_shortcut(options: &[CommandDataOption]) -> Option<String> {
    match options.get(0).and_then(|opt| opt.resolved.as_ref()) {
        Some(CommandDataOptionValue::String(s)) => Some(s.clone()),
        _ => None,
    }
}

fn get_link(options: &[CommandDataOption]) -> Option<String> {
    match options.get(1).and_then(|opt| opt.resolved.as_ref()) {
        Some(CommandDataOptionValue::String(s)) => Some(s.clone()),
        _ => None,
    }
}

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name("go")
        .description("Link shortener")
        .create_option(|option| {
            option
                .name("shortcut")
                .description("The name of the shortcut")
                .kind(CommandOptionType::String)
                .required(true)
        })
        .create_option(|option| {
            option
                .name("link")
                .description("The link")
                .kind(CommandOptionType::String)
                .required(false)
        })
}
