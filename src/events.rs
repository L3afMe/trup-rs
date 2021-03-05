use super::checks::*;
//use super::Config;
use anyhow::{anyhow, Context, Result};
use chrono_humanize::*;
use itertools::Itertools;
use serenity::async_trait;
use serenity::cache::Cache;
use serenity::client;
use serenity::framework::standard::macros::{check, group, help};
use serenity::framework::standard::StandardFramework;
use serenity::framework::standard::{
    help_commands, Args, CommandGroup, CommandOptions, HelpOptions, Reason,
};
use serenity::framework::standard::{macros::command, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;

use crate::{extensions::UserExt, log_errors, Config};
use indoc::indoc;

pub struct Handler;

async fn handle_showcase_post(ctx: client::Context, msg: Message) {
    log_errors! {
        if msg.attachments.is_empty() {
            msg.delete(&ctx)
                .await
                .context("Failed to delete invalid showcase submission")?;
            msg.author.direct_message(&ctx, |f| {
                f.content(indoc!("
                    Your showcase submission was detected to be invalid. If you wanna comment on a rice, use the #ricing-theming channel.
                    If this is a mistake, contact the moderators or open an issue on https://github.com/unixporn/trup
                "))
            }).await.context("Failed to send DM about invalid showcase submission")?;
        } else {
            msg.react(&ctx, ReactionType::Unicode("❤️".to_string()))
                .await
                .context("Error reacting to showcase submission with ❤️")?;
        }
    };
}

async fn handle_feedback_post(ctx: client::Context, msg: Message) {
    log_errors! {
        tokio::try_join!(
            msg.react(&ctx, ReactionType::Unicode("👍".to_string())),
            msg.react(&ctx, ReactionType::Unicode("👎".to_string())),
        ).context("Error reacting to feedback submission with 👍")?;
    };
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _ctx: client::Context, _data_about_bot: Ready) {
        println!("Trup is ready!");
    }
    async fn message(&self, ctx: client::Context, msg: Message) {
        let config = ctx.data.read().await.get::<Config>().unwrap().clone();
        if msg.author.bot {
            return;
        }
        if msg.channel_id == config.channel_showcase {
            handle_showcase_post(ctx, msg).await;
        } else if msg.channel_id == config.channel_feedback {
            handle_feedback_post(ctx, msg).await;
        }
    }

    async fn guild_member_addition(
        &self,
        ctx: client::Context,
        guild_id: GuildId,
        new_member: Member,
    ) {
        let config = ctx.data.read().await.get::<Config>().unwrap().clone();
        if config.guild != guild_id {
            return;
        }

        log_errors! {
            config
                .channel_bot_traffic
                .send_message(&ctx, |m| {
                    m.embed(|e| {
                        e.author(|a| {
                            a.name("Member Join");
                            a.icon_url(
                                new_member
                                    .user
                                    .avatar_or_default()
                            )
                        });
                        e.title(new_member.user.name_with_disc_and_id());
                        e.field("Account Creation Date", HumanTime::from(new_member.user.created_at()).to_text_en(Accuracy::Precise, Tense::Past), false);
                        if let Some(join_date) = new_member.joined_at {
                            e.field("Join Date", HumanTime::from(join_date).to_text_en(Accuracy::Precise, Tense::Past), false);
                        }
                        e
                    })
                })
                .await?;
        };
    }

    async fn guild_member_removal(
        &self,
        ctx: client::Context,
        guild_id: GuildId,
        user: User,
        _member: Option<Member>,
    ) {
        let config = ctx.data.read().await.get::<Config>().unwrap().clone();
        if config.guild != guild_id {
            return;
        }

        log_errors! {
            config
                .channel_bot_traffic
                .send_message(&ctx, |m| {
                    m.embed(|e| {
                        e.author(|a| {
                            a.name("Member Leave");
                            a.icon_url(user.avatar_or_default())
                        });
                        e.title(user.name_with_disc_and_id());
                        e.field("Leave Date", HumanTime::from(chrono::Utc::now()).to_text_en(Accuracy::Precise, Tense::Past), false);
                        //e.field("Account Creation Date", HumanTime::from(user.created_at()).to_text_en(Accuracy::Precise, Tense::Past), false);
                        e
                    })
                })
                .await?;
        };
    }

    async fn message_update(
        &self,
        ctx: client::Context,
        old_if_available: Option<Message>,
        _new: Option<Message>,
        event: MessageUpdateEvent,
    ) {
        let config = ctx.data.read().await.get::<Config>().unwrap().clone();

        if Some(config.guild) != event.guild_id
            || event.edited_timestamp.is_none()
            || event.author.as_ref().map(|x| x.bot).unwrap_or(false)
        {
            return;
        };

        log_errors! {
            let msg = event.channel_id.message(&ctx, event.id).await?;

            let channel_name = event
                .channel_id
                .name(&ctx)
                .await
                .unwrap_or("unknown".to_string());

            event
                .channel_id
                .send_message(&ctx, |m| {
                    m.embed(|e| {
                        e.author(|a| {
                            a.name("Message Edit").icon_url(msg.author.avatar_or_default())
                        });
                        e.title(msg.author.name_with_disc_and_id());
                        e.description(indoc::formatdoc!("
                                **Before:**
                                {}

                                **Now:**
                                {}

                                [(context)]({})
                            ",
                            old_if_available
                                    .map(|old| old.content)
                                    .unwrap_or("<Unavailable>".to_string()),
                                event.content.clone().unwrap_or("<Unavailable>".to_string()),
                                msg.link()
                        ));
                        if let Some(edited_timestamp) = event.edited_timestamp {
                            e.timestamp(&edited_timestamp);
                        }
                        e.footer(|f| f.text(format!("#{}", channel_name)))
                    })
                })
                .await?;

        };
    }

    //async fn reaction_add(&self, ctx: client::Context, event: Reaction) {
    //if event.user(&ctx).await.unwrap().bot {
    //return;
    //}
    //// todo user disambiguation

    //let message = event.message(&ctx).await.unwrap();

    //if message.author.bot
    //&& message
    //.embeds
    //.iter()
    //.any(|embed| embed.title.unwrap_or_default().starts_with("Poll:"))
    //{
    //let channel = message.channel(&ctx).await.unwrap();
    //}
    //}
}
