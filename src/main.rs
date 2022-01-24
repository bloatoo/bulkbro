use rwcord::{
    async_trait,
    discord::{
        embed::{Embed, EmbedField, EmbedImage},
        Message,
    },
    Client, Context, Handler,
};
use serde_json::Value;

use rand::Rng;
use tokio_postgres::{connect, Client as PSQClient, NoTls};

mod command;
use command::Command;

use command::WorkoutsCommand;

use url::Url;

struct EventHandler;

const MUSCLE_GROUPS: &[&'static str] = &[
    "triceps", "biceps", "chest", "back", "legs", "calves", "abs",
];

pub struct State {
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
                "exercises" => match args.get(0) {
                    None => {
                        let embed = Embed::new()
                            .title("Exercise commands")
                            .color("#c8ccd4")
                            .add_field(EmbedField {
                                name: "`bb exercise query`".into(),
                                value: "Query information about exercises (via muscle group (ex: `bb exercise query triceps`) or via exercise name (ex: `bb exercise query diamond push up`))".into(),
                                inline: false
                            })
                            .add_field(EmbedField {
                                name: "`bb exercise view`".into(),
                                value: "View detailed information about an exercise (ex: `bb exercise view diamond push up`)".into(),
                                inline: false
                            });

                        msg.reply(ctx.http(), embed).await.unwrap();
                    }

                    Some(x) => match *x {
                        "query" => {
                            let state = ctx.state().read().await;

                            let mut embed = Embed::new()
                                .title("Exercises query results")
                                .color("#c8ccd4");

                            let query = args[1..].join(" ").to_lowercase();

                            if MUSCLE_GROUPS.contains(&query.trim()) {
                                let rows = state
                                .db
                                .query(
                                    "SELECT name, description FROM exercises WHERE muscles_worked[1] = $1",
                                    &[&args[1]],
                                )
                                .await
                                .unwrap();

                                for r in rows {
                                    let name: String = r.get(0);
                                    let description: String = r.get(1);

                                    embed = embed.add_field(EmbedField {
                                        name,
                                        value: description,
                                        inline: false,
                                    });
                                }
                            } else {
                                let rows = state.db.query("SELECT name, muscles_worked, description FROM exercises WHERE LOWER(name) LIKE '%' || $1 || '%';", &[&query]).await.unwrap();

                                for r in rows {
                                    let name: String = r.get(0);
                                    let description: String = r.get(2);
                                    let mut muscles_worked_string = String::new();

                                    let muscles_worked_vec: Vec<String> = r.get(1);

                                    muscles_worked_vec.iter().for_each(|x| {
                                        muscles_worked_string.push_str(&format!("{}, ", x))
                                    });

                                    muscles_worked_string.pop();
                                    muscles_worked_string.pop();

                                    embed = embed.add_field(EmbedField {
                                        name: format!("{} | {}", name, muscles_worked_string),
                                        value: description,
                                        inline: false,
                                    });
                                }
                            }

                            msg.reply(ctx.http(), embed).await.unwrap();
                        }

                        "view" => {
                            let state = ctx.state().read().await;

                            let query = args[1..].join(" ").to_lowercase();
                            let rows = state.db.query("SELECT name, muscles_worked, description, image_url FROM exercises WHERE LOWER(name) LIKE '%' || $1 || '%';", &[&query]).await.unwrap();

                            if rows.len() == 0 {
                                return;
                            }

                            let r = &rows[0];

                            let name: String = r.get(0);
                            let muscles_worked: Vec<String> = r.get(1);
                            let description: String = r.get(2);
                            let image_url: String = r.get(3);

                            let mut final_desc = format!(
                                "{}\nMuscles worked: {}.",
                                description,
                                muscles_worked.join(", ")
                            );

                            let embed = Embed::new()
                                .title(name)
                                .color("#c8ccd4")
                                .description(final_desc)
                                .image(EmbedImage {
                                    url: image_url,
                                    ..Default::default()
                                });

                            msg.reply(ctx.http(), embed).await.unwrap();
                        }
                        _ => (),
                    },
                },
                "workouts" => {
                    WorkoutsCommand::exec(ctx, &msg, args).await.unwrap();
                }

                "music" => {
                    let state = ctx.state().read().await;

                    match args[0] {
                        "add" => {
                            let mut final_url = String::new();

                            let url = match Url::parse(args[1]) {
                                Ok(url) => url,
                                Err(_) => {
                                    msg.reply(ctx.http(), "Invalid URL.").await.unwrap();
                                    return;
                                }
                            };

                            let mut valid = false;

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
                                .color("#c8ccd4");

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
