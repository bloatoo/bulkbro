use rwcord::async_trait;
use rwcord::discord::{
    embed::{Embed, EmbedField},
    Message,
};

use rwcord::Context;
use std::error::Error;

use crate::State;

#[async_trait]
pub trait Command<T> {
    async fn exec(ctx: Context<T>, msg: &Message, args: Vec<&str>) -> Result<(), Box<dyn Error>>;
}

pub struct WorkoutsCommand {}

#[async_trait]
impl Command<State> for WorkoutsCommand {
    async fn exec(
        ctx: Context<State>,
        msg: &Message,
        args: Vec<&str>,
    ) -> Result<(), Box<dyn Error>> {
        match args[0] {
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
        }

        Ok(())
    }
}
