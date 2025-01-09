use crate::repo::database::Guild;
use crate::{Context, Error};

use poise::serenity_prelude as serenity;

#[poise::command(
    prefix_command,
    slash_command,
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn setchannel(ctx: Context<'_>) -> Result<(), Error> {
    let guild = Guild {
        id: ctx.guild().unwrap().id.get().try_into().unwrap(),
        channel: ctx.channel_id().get().try_into().unwrap(),
    };
    match ctx.data().db.update_guild(&guild).await {
        Ok(_) => {
            let response = format!("Bark Bark!!! You've successfully shown me where my home is!!\nPlease make sure I have permissions to masseg in this channel");
            ctx.say(response).await?;
            Ok(())
        }
        Err(_) => {
            let response = format!("Server Error");
            ctx.say(response).await?;
            Ok(())
        }
    }
}
