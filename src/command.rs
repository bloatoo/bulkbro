use rwcord::async_trait;
use rwcord::Context;
use std::error::Error;

#[async_trait]
pub trait Command<T> {
    async fn exec(ctx: Context<T>, args: Vec<&str>) -> Result<(), Box<dyn Error>>;
}
