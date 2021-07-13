use std::collections::HashSet;

use serenity::{
	prelude::*,
};
use serenity::{
	framework::standard::{
		Args,
		CommandGroup,
		CommandResult,
		DispatchError,
		help_commands,
		HelpOptions,
		macros::{group, help, hook},
	},
	model::{
		channel::Message,
		id::UserId,
	},
};

use crate::commands::meta::*;

#[group]
#[commands(ping, latency)]
pub struct General;

// The framework provides two built-in help commands for you to use.
// But you can also make your own customized help command that forwards
// to the behaviour of either of them.
#[help]
// This replaces the information that a user can pass
// a command-name as argument to gain specific information about it.
#[individual_command_tip = "If you want more information about a specific command, just pass the command as argument."]
// Some arguments require a `{}` in order to replace it with contextual information.
// In this case our `{}` refers to a command's name.
#[command_not_found_text = "Could not find command `{}`."]
// Define the maximum Levenshtein-distance between a searched command-name
// and commands. If the distance is lower than or equal the set distance,
// it will be displayed as a suggestion.
// Setting the distance to 0 will disable suggestions.
#[max_levenshtein_distance(5)]
// When you use sub-groups, Serenity will use the `indention_prefix` to indicate
// how deeply an item is indented.
// The default value is "-", it will be changed to "+".
#[indention_prefix = "+"]
// On another note, you can set up the help-menu-filter-behaviour.
// Here are all possible settings shown on all possible options.
// First case is if a user lacks permissions for a command, we can hide the command.
#[lacking_permissions = "Hide"]
// If the user is nothing but lacking a certain role, we just display it hence our variant is `Nothing`.
#[lacking_role = "Nothing"]
// The last `enum`-variant is `Strike`, which ~~strikes~~ a command.
#[wrong_channel = "Strike"]
// Serenity will automatically analyse and generate a hint/tip explaining the possible
// cases of ~~strikethrough-commands~~, but only if
// `strikethrough_commands_tip_in_{dm, guild}` aren't specified.
// If you pass in a value, it will be displayed instead.
pub async fn my_help(
	context: &Context,
	msg: &Message,
	args: Args,
	help_options: &'static HelpOptions,
	groups: &[&'static CommandGroup],
	owners: HashSet<UserId>,
) -> CommandResult {
	let _ = help_commands::with_embeds(context, msg, args, help_options, groups, owners).await;
	Ok(())
}

#[hook]
pub async fn before(_ctx: &Context, msg: &Message, command_name: &str) -> bool {
	println!("Got command '{}' by user '{}'", command_name, msg.author.name);

	true // if `before` returns false, command processing doesn't happen.
}

#[hook]
pub async fn after(_ctx: &Context, _msg: &Message, command_name: &str, command_result: CommandResult) {
	match command_result {
		Ok(()) => println!("Processed command '{}'", command_name),
		Err(why) => println!("Command '{}' returned error {:?}", command_name, why),
	}
}

#[hook]
pub async fn unknown_command(_ctx: &Context, msg: &Message, unknown_command_name: &str) {
	println!("Could not find command named '{}'\n(Message content: \"{}\")", unknown_command_name, msg.content);
}

#[hook]
pub async fn normal_message(_ctx: &Context, msg: &Message) {
	println!("Processed non Command message: '{}'", msg.content);
}

#[hook]
pub async fn dispatch_error(ctx: &Context, msg: &Message, error: DispatchError) {
	if let DispatchError::Ratelimited(info) = error {
		// We notify them only once.
		if info.is_first_try {
			let _ = msg
				.channel_id
				.say(&ctx.http, &format!("Try this again in {} seconds.", info.as_secs()))
				.await;
		}
	}
}

#[hook]
pub async fn delay_action(ctx: &Context, msg: &Message) {
	// You may want to handle a Discord rate limit if this fails.
	let _ = msg.react(ctx, '⏱').await;
}