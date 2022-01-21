use rand::Rng;
use rwcord::{
    async_trait,
    discord::{
        embed::{Embed, EmbedField},
        Message,
    },
    Client, Context, Handler,
};
use serde_json::Value;

use tokio_postgres::{connect, Client as PSQClient, NoTls};

mod command;
use command::Command;

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
                "workouts" => match args[0] {
                    "view" => {
                        let state = ctx.state().read().await;
                        let rows = state
                            .db
                            .query(
                                "SELECT title FROM workouts WHERE author_id = $1",
                                &[msg.author().id()],
                            )
                            .await
                            .unwrap();

                        let mut embed = Embed::new().title("Your Workouts").color("#bf616a");

                        for row in rows {
                            let workout_title: String = row.get(0);

                            let field = EmbedField {
                                name: workout_title,
                                value: "Some text".into(),
                                inline: true,
                            };

                            embed = embed.add_field(field);
                        }

                        msg.reply(ctx.http(), embed).await.unwrap();
                    }
                    _ => (),
                },

                "set" => {
                    let state = ctx.state().read().await;

                    match args[0] {
                        "squat" => {
                            let rows = state
                                .db
                                .query(
                                    "SELECT max_squat FROM users WHERE id = $1",
                                    &[msg.author().id()],
                                )
                                .await
                                .unwrap();

                            let squat = args[1].parse::<i32>().unwrap();

                            if let Some(_) = rows.get(0) {
                                state
                                    .db
                                    .query(
                                        "UPDATE users SET max_squat = $1 WHERE id = $2",
                                        &[&squat, msg.author().id()],
                                    )
                                    .await
                                    .unwrap();
                            } else {
                                state
                                    .db
                                    .query(
                                        "INSERT INTO users(id, max_squat) VALUES ($1, $2)",
                                        &[msg.author().id(), &squat],
                                    )
                                    .await
                                    .unwrap();
                            }

                            let embed = Embed::new()
                                .title(":white_check_mark: Success!")
                                .description(format!("Your squat PR has been set to {} kg.", squat))
                                .color("#81a1c1");

                            msg.reply(ctx.http(), embed).await.unwrap();
                        }
                        _ => (),
                    }
                }
                _ => (),
            }
        }
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
