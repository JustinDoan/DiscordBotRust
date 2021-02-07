use serde::Deserialize;
use serenity::{
    async_trait,
    builder::CreateEmbedFooter,
    model::{
        channel::{EmbedFooter, Message},
        gateway::{Activity, Ready},
        id::{ChannelId, GuildId},
    },
    prelude::*,
};
use std::collections::HashMap;
use std::time::Instant;
use std::{
    env,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use sysinfo::{System, SystemExt};

struct Handler {
    is_loop_running: AtomicBool,
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        let args: Vec<String> = msg.content.split(" ").map(|s| s.to_string()).collect();
        if args[0] == "!ping" {
            let start_time = Instant::now();
            let author = msg.author;
            let author_nickname = author.nick_in(&ctx.http, msg.guild_id.unwrap()).await;
            if let Err(why) = msg
                .channel_id
                .say(
                    &ctx.http,
                    format!(
                        "I recieved a ping! \n Their name in this guild is {} \n This message took {} seconds to generate.",
                        &author_nickname.unwrap(),
                        start_time.elapsed().as_nanos() as f64 / 1000000000 as f64
                    ),
                )
                .await
            {
                println!("Error sending message: {:?}", why);
            }
        }
        if args[0] == "!islive" {
            let start_time = Instant::now();
            let ctx = Arc::new(ctx);
            let username = &args[1];
            check_if_user_is_live(ctx, username.to_string(), start_time).await
        }
    }

    // Set a handler to be called on the `ready` event. This is called when a
    // shard is booted, and a READY payload is sent by Discord. This payload
    // contains data like the current user's guild Ids, current user data,
    // private channels, and more.
    //
    // In this case, just print what the current user's username is.
    async fn ready(&self, ctx: Context, ready: Ready) {
        // it's safe to clone Context, but Arc is cheaper for this use case.
        // Untested claim, just theoretically. :P
        // let ctx = Arc::new(ctx);

        // // We need to check that the loop is not already running when this event triggers,
        // // as this event triggers every time the bot enters or leaves a guild, along every time the
        // // ready shard event triggers.
        // //
        // // An AtomicBool is used because it doesn't require a mutable reference to be changed, as
        // // we don't have one due to self being an immutable reference.
        // if !self.is_loop_running.load(Ordering::Relaxed) {
        //     // We have to clone the Arc, as it gets moved into the new thread.
        //     let ctx1 = Arc::clone(&ctx);
        //     // tokio::spawn creates a new green thread that can run in parallel with the rest of
        //     // the application.
        //     tokio::spawn(async move {
        //         loop {
        //             // We clone Context again here, because Arc is owned, so it moves to the
        //             // new function.
        //             log_system_load(Arc::clone(&ctx1)).await;
        //             tokio::time::sleep(Duration::from_secs(15)).await;
        //         }
        //     });
        //     // Should be able to use this to pass this into a global rocket instance i can init from this function.
        //     //https://stackoverflow.com/questions/55384204/how-can-i-pass-a-variable-initialized-in-main-to-a-rocket-route-handler
        //     // Now that the loop is running, we set the bool to true
        //     self.is_loop_running.swap(true, Ordering::Relaxed);
        // }
        println!("{} is connected!", ready.user.name);
    }
}

#[derive(Deserialize)]
struct AccessInformation {
    access_token: String,
    // refresh_token: String,
    // expires_in: u64,
    // scope: Vec<String>,
    // token_type: String,
}

#[derive(Deserialize)]
struct IsLive {
    data: Vec<UserInformation>,
}

#[derive(Deserialize)]
struct UserInformation {
    is_live: bool,
}

async fn check_if_user_is_live(ctx: Arc<Context>, username: String, start_time: Instant) {
    let client = reqwest::Client::new();

    let authentication_call = client
    .post("https://id.twitch.tv/oauth2/token?client_id=client-id-placeholder&client_secret=client-secret-placeholder&grant_type=client_credentials")
    .send()
    .await
    .unwrap();

    let json_response: AccessInformation = authentication_call.json().await.unwrap();
    let url = format!(
        "https://api.twitch.tv/helix/search/channels?query={}",
        username
    );
    let res = client
        .get(&url.to_string())
        .header("client-id", "client-id-placeholder")
        .header(
            "Authorization",
            format!("Bearer {}", json_response.access_token),
        )
        .send()
        .await;

    let user_json_response: IsLive = res.unwrap().json().await.unwrap();
    let is_user_live: &str;
    if user_json_response.data[0].is_live == false {
        is_user_live = "not live";
    } else {
        is_user_live = "live";
    }

    if let Err(why) = ChannelId(699288524247007352)
        .send_message(&ctx, |m| {
            m.embed(|e| {
                e.title(format!("Is {} live on Twitch?", username));
                e.field(
                    format!("{}'s status", username),
                    format!("{} is currently {}.", username, is_user_live),
                    false,
                );
                e.footer(|f| {
                    f.text(format!(
                        "Message Generated in {} seconds.",
                        start_time.elapsed().as_nanos() as f64 / 1000000000 as f64
                    ))
                });
                e
            })
        })
        .await
    {
        eprintln!("Error sending message: {:?}", why);
    };
}

async fn log_system_load(ctx: Arc<Context>) {
    let s = System::new_all();
    let load_avg = s.get_load_average();

    // We can use ChannelId directly to send a message to a specific channel; in this case, the
    // message would be sent to the #testing channel on the discord server.
    if let Err(why) = ChannelId(699288524247007352)
        .send_message(&ctx, |m| {
            m.embed(|e| {
                e.title("System Resource Load");
                e.field("CPU Load Average", format!("{:.2}%", load_avg.one), false);
                e.field(
                    "Used Memory",
                    format!("{} MB", s.get_used_memory() / 1000),
                    false,
                );
                e
            })
        })
        .await
    {
        eprintln!("Error sending message: {:?}", why);
    };
}

#[tokio::main]
async fn main() {
    // Configure the client with your Discord bot token in the environment.
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    // Create a new instance of the Client, logging in as a bot. This will
    // automatically prepend your bot token with "Bot ", which is a requirement
    // by Discord for bot users.
    let mut client = Client::builder(&token)
        .event_handler(Handler {
            is_loop_running: AtomicBool::new(false),
        })
        .await
        .expect("Err creating client");

    // Finally, start a single shard, and start listening to events.
    //
    // Shards will automatically attempt to reconnect, and will perform
    // exponential backoff until it reconnects.
    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
