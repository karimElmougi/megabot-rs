use crate::config::{Config, Feature};

use std::sync::Arc;

use parking_lot::RwLock;
use serenity::async_trait;
use serenity::model::channel::Reaction;
use serenity::model::gateway::Ready;
use serenity::model::prelude::interaction::{Interaction, InteractionResponseType};
use serenity::model::prelude::{ChannelId, GuildId, MessageId, ReactionType, RoleId, UserId};
use serenity::prelude::*;

pub async fn run(token: String, guild_id: GuildId, config: Arc<RwLock<Config>>) {
    // Set gateway intents, which decides what events the bot will be notified about
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::GUILD_MESSAGE_REACTIONS
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    // Create a new instance of the Client, logging in as a bot. This will
    // automatically prepend your bot token with "Bot ", which is a requirement
    // by Discord for bot users.
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler { guild_id, config })
        .await
        .expect("Err creating client");

    // Finally, start a single shard, and start listening to events.
    //
    // Shards will automatically attempt to reconnect, and will perform
    // exponential backoff until it reconnects.
    if let Err(e) = client.start().await {
        println!("Client error: {:?}", e);
    }
}

struct Handler {
    guild_id: GuildId,
    config: Arc<RwLock<Config>>,
}

#[async_trait]
impl EventHandler for Handler {
    async fn reaction_add(&self, ctx: Context, reaction: Reaction) {
        if !self.config.read().enabled_features.contains(&Feature::Pins) {
            return;
        }

        let Reaction {
            emoji,
            member,
            user_id,
            channel_id,
            message_id,
            ..
        } = reaction;

        if is_pin_emoji(emoji) {
            let user_roles = match (member, user_id) {
                (Some(member), _) => member.roles,
                (None, Some(user_id)) => fetch_user_roles(&ctx, user_id, self.guild_id).await,
                _ => return log::error!("No member info for pin reaction"),
            };

            if has_allowed_role(&user_roles, &self.config.read().pin_roles) {
                log::info!("Pinning message {}", reaction.message_id);
                if let Err(e) = pin_message(ctx, channel_id, message_id).await {
                    log::error!("Unable to pin message: {e}");
                }
            }
        }
    }

    async fn reaction_remove(&self, ctx: Context, reaction: Reaction) {
        if !self.config.read().enabled_features.contains(&Feature::Pins) {
            return;
        }

        let Reaction {
            emoji,
            member,
            user_id,
            channel_id,
            message_id,
            ..
        } = reaction;

        if is_pin_emoji(emoji) {
            let user_roles = match (member, user_id) {
                (Some(member), _) => member.roles,
                (None, Some(user_id)) => fetch_user_roles(&ctx, user_id, self.guild_id).await,
                _ => return log::error!("No member info for pin reaction"),
            };

            if has_allowed_role(&user_roles, &self.config.read().pin_roles) {
                log::info!("Unpinning message {}", reaction.message_id);
                if let Err(e) = unpin_message(ctx, channel_id, message_id).await {
                    log::error!("Unable to unpin message: {e}");
                }
            }
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            log::info!("Received {} command", command.data.name);

            let response_data = match command.data.name.as_str() {
                "ping" => "pong",
                _ => "command not yet implemented",
            };

            let result = command
                .create_interaction_response(&ctx.http, |response| {
                    response
                        .kind(InteractionResponseType::ChannelMessageWithSource)
                        .interaction_response_data(|message| message.content(response_data))
                })
                .await;

            if let Err(e) = result {
                log::error!("Unable to respond to command: {e}");
            }
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        log::info!("{} is connected!", ready.user.name);

        let result = self
            .guild_id
            .set_application_commands(&ctx.http, |commands| {
                commands.create_application_command(|command| {
                    command.name("ping").description("A ping command")
                })
            })
            .await;

        if let Err(e) = result {
            log::error!("Unable to create commands: {e}");
            std::process::exit(1);
        }
    }
}

async fn fetch_user_roles(ctx: &Context, user_id: UserId, guild_id: GuildId) -> Vec<RoleId> {
    match ctx
        .http
        .get_member(guild_id.0, user_id.0)
        .await
        .map(|m| m.roles)
    {
        Ok(roles) => roles,
        Err(e) => {
            log::error!("Unable to fetch roles for user {user_id}: {e}");
            Vec::new()
        }
    }
}

async fn pin_message(
    ctx: Context,
    channel_id: ChannelId,
    message_id: MessageId,
) -> serenity::Result<()> {
    let message = ctx.http.get_message(channel_id.0, message_id.0).await?;

    message.pin(ctx.http).await
}

async fn unpin_message(
    ctx: Context,
    channel_id: ChannelId,
    message_id: MessageId,
) -> serenity::Result<()> {
    let message = ctx.http.get_message(channel_id.0, message_id.0).await?;

    message.unpin(ctx.http).await
}

fn has_allowed_role(roles: &[RoleId], allowed_roles: &[RoleId]) -> bool {
    for role in roles {
        if allowed_roles.contains(role) {
            return true;
        }
    }
    false
}

fn is_pin_emoji(reaction_type: ReactionType) -> bool {
    const PIN_EMOJI: &str = "📌";

    match reaction_type {
        ReactionType::Unicode(emoji) => emoji == PIN_EMOJI,
        _ => false,
    }
}
