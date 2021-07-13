use std::fmt::{Display, Formatter};
use std::time::{SystemTime, UNIX_EPOCH};

use serenity::futures::executor::block_on;
use serenity::model::channel::Message;
use serenity::model::user::User;
use sqlx::PgPool;

pub struct MessageDb {
	message_id: i64,
	author: User,
	channel_id: i64,
	guild_id: Option<i64>,
	content: String,
	deleted: bool,
	timestamp: i64,
}

impl MessageDb {
	pub fn from_message(message: Message) -> MessageDb {
		let id = message.guild_id.map(|guild_id| i64::from(guild_id));
		MessageDb {
			message_id: i64::from(message.id),
			author: message.author,
			channel_id: message.channel_id.0 as i64,
			guild_id: id,
			content: message.content,
			deleted: false,
			timestamp: SystemTime::now()
				.duration_since(UNIX_EPOCH)
				.expect("Time got fucked")
				.as_millis() as i64,
		}
	}

	pub async fn write_to_db(&self, connection_pool: &PgPool) {
		let user_insert_query = sqlx::query(
			"INSERT INTO users VALUES ($1, $2) ON CONFLICT DO NOTHING")
			.bind(i64::from(self.author.id))
			.bind(&self.author.name);

		let query = sqlx::query(
			"INSERT INTO messages (message_id, author, channel_id, guild_id, content, deleted, timestamp)
				VALUES ($1, $2, $3, $4, $5, $6, $7)")
			.bind(self.message_id)
			.bind(i64::from(self.author.id))
			.bind(self.channel_id)
			.bind(self.guild_id)
			.bind(&self.content)
			.bind(self.deleted)
			.bind(self.timestamp);
		let result2 = block_on(user_insert_query.execute(connection_pool));
		let result = query.execute(connection_pool).await;

		match result {
			Ok(_) => println!("No error"),
			Err(e) => println!("Error: {}", e)
		}
		match result2 {
			Ok(_) => println!("No error"),
			Err(e) => println!("Error: {}", e)
		}
	}

	pub async fn mark_deleted(message_id: i64, connection_pool: &PgPool) {
		let mark_deleted_query = sqlx::query(
			"UPDATE messages SET deleted = true WHERE message_id = $1")
			.bind(message_id);

		let result = mark_deleted_query.execute(connection_pool).await;

		match result {
			Ok(_) => println!("No error"),
			Err(e) => println!("Error: {}", e)
		}
	}

	pub async fn update_message(message_id: &i64, new_content: &String, connection_pool: &PgPool) {
		let insert_update_query = sqlx::query(
			"INSERT INTO updated_messages (message_id, new_content, modify_timestamp)
				 VALUES ($1, $2, $3)")
			.bind(message_id)
			.bind(new_content)
			.bind(SystemTime::now()
					.duration_since(UNIX_EPOCH)
					.expect("Time got fucked")
					.as_millis() as i64
			);

		let result = insert_update_query.execute(connection_pool).await;

		if let Err(why) = result {
			println!("Error: {:?}", why)
		}
	}
}

impl Display for MessageDb {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		let guild_id = match self.guild_id {
			None => 0,
			Some(id) => id,
		};

		write!(f,
			   "m{{ message_id: {}, author: {}, channel_id: {}, guild_id: {}, content: {} }}",
			   self.message_id,
			   self.author,
			   self.channel_id,
			   guild_id,
			   self.content
		)
	}
}