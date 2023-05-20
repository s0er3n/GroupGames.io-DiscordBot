use std::collections::HashSet;
use std::env;
use std::ops::Add;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use dotenv::dotenv;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use serenity::async_trait;
use serenity::framework::standard::macros::{command, group};
use serenity::framework::standard::{CommandResult, StandardFramework};
use serenity::model::channel::Message;
use serenity::model::id::ChannelId;
use serenity::model::prelude::{Activity, GuildId};
use serenity::prelude::*;

#[group]
#[commands(ping)]
struct General;

struct Handler {
    is_loop_running: AtomicBool,
}

impl Handler {
    fn new() -> Self {
        Self {
            is_loop_running: AtomicBool::new(false),
        }
    }
}

use serde_derive::Deserialize;
use serde_derive::Serialize;
use serde_json::Value;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TwitchResponse {
    pub data: Vec<Stream>,
    pub pagination: Pagination,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stream {
    pub id: String,
    #[serde(rename = "user_id")]
    pub user_id: String,
    #[serde(rename = "user_login")]
    pub user_login: String,
    #[serde(rename = "user_name")]
    pub user_name: String,
    #[serde(rename = "game_id")]
    pub game_id: String,
    #[serde(rename = "game_name")]
    pub game_name: String,
    #[serde(rename = "type")]
    pub type_field: String,
    pub title: String,
    #[serde(rename = "viewer_count")]
    pub viewer_count: i64,
    #[serde(rename = "started_at")]
    pub started_at: String,
    pub language: String,
    #[serde(rename = "thumbnail_url")]
    pub thumbnail_url: String,
    #[serde(rename = "tag_ids")]
    pub tag_ids: Vec<Value>,
    pub tags: Vec<String>,
    #[serde(rename = "is_mature")]
    pub is_mature: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pagination {
    pub cursor: String,
}

#[async_trait]
impl EventHandler for Handler {
    async fn cache_ready(&self, ctx: Context, _guilds: Vec<GuildId>) {
        let ctx = Arc::new(ctx);
        if !self.is_loop_running.load(Ordering::Relaxed) {
            let ctx = ctx.clone();
            tokio::spawn(async move {
                let client = reqwest::Client::new();

                let mut headers = HeaderMap::new();
                headers.insert(
                    AUTHORIZATION,
                    HeaderValue::from_str(&format!(
                        "Bearer {}",
                        env::var("TWITCH_TOKEN").expect("TWITCH_TOKEN")
                    ))
                    .expect("Value"),
                );
                let client_id = env::var("CLIENT_ID").expect("CLIENT_ID");

                headers.insert(
                    "Client-Id",
                    HeaderValue::from_str(&client_id).expect("CLIENT_ID"),
                );

                let mut current_streamers: HashSet<String> = HashSet::new();

                loop {
                    let res = client
                        .get("https://api.twitch.tv/helix/streams?game_id=32399&language=en")
                        .headers(headers.clone())
                        .send()
                        .await;
                    if let Ok(res) = res {
                        let data: TwitchResponse = res.json().await.expect("json");

                        let mut new_streamers: HashSet<String> = HashSet::new();
                        for streamer in data.data {
                            new_streamers.insert(streamer.user_name);
                        }
                        let diff = new_streamers.difference(&current_streamers);
                        for streamer in diff {
                            ChannelId(
                                env::var("DISCORD_CHANNEL_ID")
                                    .expect("DISCORD")
                                    .parse::<u64>()
                                    .expect("DISCORD"),
                            )
                            .send_message(&ctx.http, |m| {
                                m.content(format!("https://twitch.tv/{}", streamer))
                            })
                            .await;
                        }
                        current_streamers = new_streamers;
                    }
                    tokio::time::sleep(Duration::from_secs(60)).await;
                }
            });
            self.is_loop_running.swap(true, Ordering::Relaxed);
        }
    }
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    let framework = StandardFramework::new()
        .configure(|c| c.prefix("~")) // set the bot's prefix to "~"
        .group(&GENERAL_GROUP);

    // Login with a bot token from the environment
    let token = env::var("TOKEN").expect("TOKEN");
    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(token, intents)
        .event_handler(Handler::new())
        .framework(framework)
        .await
        .expect("Error creating client");

    // start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        println!("An error occurred while running the client: {:?}", why);
    }
}

#[command]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    msg.reply(ctx, "Pong!").await?;

    Ok(())
}
