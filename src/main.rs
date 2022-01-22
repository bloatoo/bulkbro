use rwcord::{
    async_trait,
    discord::{
        embed::{Embed, EmbedField},
        Message,
    },
    Client, Context, Handler,
};
use serde_json::Value;

use rand::Rng;
use tokio_postgres::{connect, Client as PSQClient, NoTls};

mod command;
use command::Command;

use url::Url;

const VALID_HOSTS: &[&'static str] = &[
    "youtube.com",
    "www.youtube.com",
    "youtu.be",
    "m.youtube.com",
];

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

            let mut final_url = String::new();

            match cmd {
                "music" => {
                    let state = ctx.state().read().await;

                    match args[0] {
                        "add" => {
                            let url = match Url::parse(args[1]) {
                                Ok(url) => url,
                                Err(_) => {
                                    msg.reply(ctx.http(), "Invalid URL.").await.unwrap();
                                    return;
                                }
                            };

                            let mut valid = false;

                            for h in VALID_HOSTS {
                                if url.host_str() == Some(h) {
                                    valid = true;
                                }
                            }

                            if url.scheme() != "https" && url.scheme() != "http" {
                                valid = false;
                            }

                            match url.host_str() {
                                Some("youtube.com")
                                | Some("www.youtube.com")
                                | Some("m.youtube.com") => {
                                    if url.path() != "/watch" {
                                        valid = false;
                                    }

                                    match url.query() {
                                        Some(q) => {
                                            let id_start = q.find("v=").unwrap();
                                            let t_str = &q[id_start + 2..];
                                            let mut v_str = String::new();

                                            for c in t_str.chars() {
                                                if c.is_ascii_alphabetic() || c.is_ascii_digit() {
                                                    v_str.push(c);
                                                } else {
                                                    break;
                                                }
                                            }

                                            final_url =
                                                format!("https://youtube.com/watch?v={}", v_str);
                                        }

                                        None => {
                                            valid = false;
                                        }
                                    }
                                }

                                Some("youtu.be") => {
                                    let mut id = url.path().to_string();
                                    id.remove(0);
                                    final_url = format!("https://youtube.com/watch?v={}", id);
                                }
                                Some(&_) => {}
                                None => {
                                    valid = false;
                                }
                            }

                            let res = reqwest::get(format!(
                                "https://www.youtube.com/oembed?format=json&url={}",
                                url
                            ))
                            .await
                            .unwrap();

                            if res.status() != 200 {
                                valid = false;
                            } else {
                                valid = true;
                            }

                            if !valid {
                                msg.reply(ctx.http(), "Invalid URL. Must be a YouTube link.")
                                    .await
                                    .unwrap();
                                return;
                            }

                            if let Err(_) = state
                                .db
                                .query("INSERT into songs(url) VALUES ($1)", &[&final_url])
                                .await
                            {
                                msg.reply(ctx.http(), "Error when adding song to the database. Maybe it already exists there?").await.unwrap();
                            } else {
                                msg.reply(ctx.http(), "Success").await.unwrap();
                            }
                        }

                        "random" => {
                            let rows = state.db.query("SELECT url FROM songs;", &[]).await.unwrap();

                            if rows.len() == 0 {
                                msg.reply(ctx.http(), "No songs in the database.")
                                    .await
                                    .unwrap();
                            }

                            let rand = if rows.len() < 2 {
                                0
                            } else {
                                rand::thread_rng().gen_range(0..rows.len())
                            };

                            let url: String = rows[rand].get(0);

                            msg.reply(ctx.http(), url).await.unwrap();
                        }

                        _ => (),
                    }
                }
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
