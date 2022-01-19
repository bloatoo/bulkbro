use rand::Rng;
use rwcord::{async_trait, discord::Message, Client, Context, Handler};
use serde_json::Value;

use tokio_postgres::{connect, Client as PSQClient, NoTls};

struct EventHandler;

struct State {
    db: PSQClient,
}

#[async_trait]
impl Handler<State> for EventHandler {
    async fn on_message_create(ctx: Context<State>, msg: Message) {
        let content = msg.content();

        if content.starts_with("bb ") {
            let mut args: Vec<&str> = content[3..].split(" ").collect();
            let cmd = args.remove(0);

            match cmd {
                "ping" => {
                    msg.reply(ctx.http(), "pong").await.unwrap();
                }
                _ => (),
            }
        }
        /*if msg.content() == "!copypasta" {
            let other_res = reqwest::get("https://www.reddit.com/r/amitheasshole/top/.json?sort=top&t=day&showmedia=false&mediaonly=false&is_self=true&limit=100").await.unwrap().text().await.unwrap();

            let json: Value = serde_json::from_str(&other_res[..]).unwrap();

            let quote = json["data"]["children"][rand::thread_rng().gen_range(0..100) as usize]
                ["data"]["selftext"]
                .as_str()
                .unwrap();

            if quote.len() > 2000 {
                msg.reply(ctx.http(), &quote[..1999]).await.unwrap();
                msg.reply(ctx.http(), &quote[1999..]).await.unwrap();
            } else {
                msg.reply(ctx.http(), quote).await.unwrap();
            }
        }*/
    }
}

#[tokio::main]
async fn main() {
    let db_host = std::env::var("DB_HOST").unwrap();
    let db_user = std::env::var("DB_USER").unwrap();
    let db_name = std::env::var("DB_NAME").unwrap();

    let token = std::env::var("TOKEN").unwrap();

    let (client, conn) = connect(
        &format!("host={} user={} dbname={}", db_host, db_user, db_name),
        NoTls,
    )
    .await
    .unwrap();

    let state = State { db: client };

    tokio::spawn(async move {
        if let Err(why) = conn.await {
            eprintln!("Connection error: {}", why);
        }
    });

    let client = Client::new(token);

    client.start::<EventHandler>(state).await.unwrap();
}
