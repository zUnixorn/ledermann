use std::env;

use dotenv::dotenv;
use serenity::{
	prelude::*,
};
use sqlx::postgres::PgPoolOptions;
use tokio;

use connection_pool::ConnectionPool;

use crate::handler::Handler;
use std::collections::HashSet;

use serenity::{
	async_trait,
	client::bridge::gateway::{ShardId, ShardManager},
	framework::standard::{
		buckets::{LimitedFor, RevertBucket},
		help_commands,
		macros::{check, command, group, help, hook},
		Args,
		CommandGroup,
		CommandOptions,
		CommandResult,
		DispatchError,
		HelpOptions,
		Reason,
		StandardFramework,
	},
	http::Http,
	model::{
		channel::{Channel, Message},
		gateway::Ready,
		id::UserId,
		permissions::Permissions,
	},
	utils::{content_safe, ContentSafeOptions},
};
use std::sync::Arc;

mod handler;
mod user_db;
mod activity_db;
mod message_db;
mod connection_pool;
mod commands;

use commands::{meta::*};
use serenity::client::bridge::gateway::GatewayIntents;

struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer {
	type Value = Arc<Mutex<ShardManager>>;
}

#[group]
#[commands(ping, latency)]
struct General;

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
async fn my_help(
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
async fn before(ctx: &Context, msg: &Message, command_name: &str) -> bool {
	println!("Got command '{}' by user '{}'", command_name, msg.author.name);

	true // if `before` returns false, command processing doesn't happen.
}

#[hook]
async fn after(_ctx: &Context, _msg: &Message, command_name: &str, command_result: CommandResult) {
	match command_result {
		Ok(()) => println!("Processed command '{}'", command_name),
		Err(why) => println!("Command '{}' returned error {:?}", command_name, why),
	}
}

#[hook]
async fn unknown_command(_ctx: &Context, msg: &Message, unknown_command_name: &str) {
	println!("Could not find command named '{}'\n(Message content: \"{}\")", unknown_command_name, msg.content);
}

#[hook]
async fn normal_message(_ctx: &Context, msg: &Message) {
	println!("Processed non Command message: '{}'", msg.content);
}

#[hook]
async fn dispatch_error(ctx: &Context, msg: &Message, error: DispatchError) {
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
async fn delay_action(ctx: &Context, msg: &Message) {
	// You may want to handle a Discord rate limit if this fails.
	let _ = msg.react(ctx, '⏱').await;
}

#[tokio::main]
async fn main() {
	dotenv().ok();

	let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

	//Connect to the Database before connecting to discord - need not start the bot if the DB is down
	let connection_pool = PgPoolOptions::new()
		.max_connections(5)
		//.idle_timeout(600)
		.connect(&env::var("DATABASE_URL").expect("Database URL Environment Variable missing")[..])
		.await
		.expect("Error while connecting to database");

	let http = Http::new_with_token(&token);

	let (owners, bot_id) = match http.get_current_application_info().await {
		Ok(info) => {
			let mut owners = HashSet::new();
			if let Some(team) = info.team {
				owners.insert(team.owner_user_id);
			} else {
				owners.insert(info.owner.id);
			}
			match http.get_current_user().await {
				Ok(bot_id) => (owners, bot_id.id),
				Err(why) => panic!("Could not access the bot id: {:?}", why),
			}
		},
		Err(why) => panic!("Could not access application info: {:?}", why),
	};

	let framework = StandardFramework::new()
		.configure(|c| c
			.with_whitespace(true)
			.on_mention(Some(bot_id))
			.prefix("~")
			// In this case, if "," would be first, a message would never
			// be delimited at ", ", forcing you to trim your arguments if you
			// want to avoid whitespaces at the start of each.
			.delimiters(vec![", ", ",", " "])
			// Sets the bot's owners. These will be used for commands that
			// are owners only.
			.owners(owners))

		// Set a function to be called prior to each command execution. This
		// provides the context of the command, the message that was received,
		// and the full name of the command that will be called.
		//
		// Avoid using this to determine whether a specific command should be
		// executed. Instead, prefer using the `#[check]` macro which
		// gives you this functionality.
		//
		// **Note**: Async closures are unstable, you may use them in your
		// application if you are fine using nightly Rust.
		// If not, we need to provide the function identifiers to the
		// hook-functions (before, after, normal, ...).
		.before(before)
		// Similar to `before`, except will be called directly _after_
		// command execution.
		.after(after)
		// Set a function that's called whenever an attempted command-call's
		// command could not be found.
		.unrecognised_command(unknown_command)
		// Set a function that's called whenever a message is not a command.
		.normal_message(normal_message)
		// Set a function that's called whenever a command's execution didn't complete for one
		// reason or another. For example, when a user has exceeded a rate-limit or a command
		// can only be performed by the bot owner.
		.on_dispatch_error(dispatch_error)
		// Can't be used more than once per 5 seconds:
		//.bucket("emoji", |b| b.delay(5)).await
		// Can't be used more than 2 times per 30 seconds, with a 5 second delay applying per channel.
		// Optionally `await_ratelimits` will delay until the command can be executed instead of
		// cancelling the command invocation.
		.bucket("complicated", |b| b.limit(2).time_span(30).delay(5)
			// The target each bucket will apply to.
			.limit_for(LimitedFor::Channel)
			// The maximum amount of command invocations that can be delayed per target.
			// Setting this to 0 (default) will never await/delay commands and cancel the invocation.
			.await_ratelimits(1)
			// A function to call when a rate limit leads to a delay.
			.delay_action(delay_action)).await
		// The `#[group]` macro generates `static` instances of the options set for the group.
		// They're made in the pattern: `#name_GROUP` for the group instance and `#name_GROUP_OPTIONS`.
		// #name is turned all uppercase
		.help(&MY_HELP)
		.group(&GENERAL_GROUP);

	let mut client = Client::builder(&token)
		.event_handler(Handler)
		.framework(framework)
		.intents(GatewayIntents::all()) //change to only require the intents we actually want
		.await
		.expect("Err creating client");

	{
		let mut data = client.data.write().await;
		data.insert::<ShardManagerContainer>(Arc::clone(&client.shard_manager));
		data.insert::<ConnectionPool>(connection_pool);
	}

	if let Err(why) = client.start().await {
		println!("Client error: {:?}", why);
	}
}