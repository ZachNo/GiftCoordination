use rocket::{Build, Rocket};
use rocket_sync_db_pools::{database, rusqlite};
use rusqlite::{params, Error};
use std::vec::Vec;

use crate::data::{Gift, List, User};

#[database("sqlite_logs")]
pub struct DbConn(rusqlite::Connection);

impl DbConn {
    const USER_TABLE: &'static str = "USERS";
    const LIST_TO_USER_TABLE: &'static str = "LIST_USERS";
    const LIST_TABLE: &'static str = "LISTS";
    const LIST_TO_GIFT_TABLE: &'static str = "LIST_ITEMS";
    const GIFT_TABLE: &'static str = "ITEMS";

    pub async fn user_uuid_from_auth_token(
        self: &DbConn,
        auth_token: String,
    ) -> Result<String, Error> {
        self.run(move |db| {
            db.query_row(
                format!(
                    "SELECT user_uuid FROM {} WHERE auth_token = ?1",
                    DbConn::USER_TABLE
                )
                .as_str(),
                params![auth_token],
                |row| row.get(0),
            )
        })
        .await
    }

    pub async fn auth_token_from_user(self: &DbConn, user_uuid: String) -> Result<String, Error> {
        self.run(move |db| {
            db.query_row(
                format!(
                    "SELECT auth_token FROM {} WHERE user_uuid = ?1",
                    DbConn::USER_TABLE
                )
                .as_str(),
                params![user_uuid],
                |row| row.get(0),
            )
        })
        .await
    }

    pub async fn user_from_email(self: &DbConn, email: String) -> Result<User, Error> {
        self.run(move |db| {
            db.query_row(
                format!(
                    "SELECT user_uuid, name, email, can_create FROM {} WHERE email = ?1",
                    DbConn::USER_TABLE
                )
                .as_str(),
                params![email],
                |row| {
                    Ok(User {
                        uuid: row.get(0).unwrap(),
                        name: row.get(1).unwrap(),
                        email: row.get(2).unwrap(),
                        can_create: row.get(3).unwrap(),
                        is_me: false,
                    })
                },
            )
        })
        .await
    }

    pub async fn user_from_uuid(
        self: &DbConn,
        user_uuid: String,
        current_uuid: String,
    ) -> Result<User, Error> {
        self.run(move |db| {
            db.query_row(
                format!(
                    "SELECT user_uuid, name, email, can_create FROM {} WHERE user_uuid = ?1",
                    DbConn::USER_TABLE
                )
                .as_str(),
                params![user_uuid],
                |row| {
                    Ok(User {
                        uuid: row.get(0).unwrap(),
                        name: row.get(1).unwrap(),
                        email: row.get(2).unwrap(),
                        can_create: row.get(3).unwrap(),
                        is_me: current_uuid.eq(&user_uuid),
                    })
                },
            )
        })
        .await
    }

    pub async fn lists_of_user(self: &DbConn, user_uuid: String) -> Result<Vec<List>, Error> {
        self.run(move |db| {
            db.prepare(
                format!(
                    "SELECT r.list_uuid, r.name, r.owner \
                FROM {} l \
                INNER JOIN {} r ON r.list_uuid = l.list_uuid \
                WHERE l.user_uuid = ?1",
                    DbConn::LIST_TO_USER_TABLE,
                    DbConn::LIST_TABLE,
                )
                .as_str(),
            )?
            .query_map(params![user_uuid], |row| {
                let owner: String = row.get(2).unwrap();
                Ok(List {
                    uuid: row.get(0).unwrap(),
                    name: row.get(1).unwrap(),
                    im_owner: user_uuid == owner.to_owned(),
                    owner,
                })
            })
            .unwrap()
            .collect::<Result<Vec<List>, _>>()
        })
        .await
    }

    pub async fn list_from_uuid(
        self: &DbConn,
        list_uuid: String,
        current_user_uuid: String,
    ) -> Result<List, Error> {
        self.run(move |db| {
            db.query_row(
                format!(
                    "SELECT list_uuid, name, owner FROM {} WHERE list_uuid = ?1",
                    DbConn::LIST_TABLE
                )
                .as_str(),
                params![list_uuid],
                |row| {
                    let owner: String = row.get(2).unwrap();
                    Ok(List {
                        uuid: row.get(0).unwrap(),
                        name: row.get(1).unwrap(),
                        im_owner: current_user_uuid == owner.to_owned(),
                        owner,
                    })
                },
            )
        })
        .await
    }

    pub async fn users_of_list(
        self: &DbConn,
        list_uuid: String,
        current_user_uuid: String,
    ) -> Result<Vec<User>, Error> {
        self.run(move |db| {
            db.prepare(
                format!(
                    "SELECT r.user_uuid, r.email, r.name, r.can_create \
                FROM {} l \
                INNER JOIN {} r ON r.user_uuid = l.user_uuid \
                WHERE l.list_uuid = ?1",
                    DbConn::LIST_TO_USER_TABLE,
                    DbConn::USER_TABLE,
                )
                .as_str(),
            )?
            .query_map(params![list_uuid], |row| {
                let user_uuid: String = row.get(0).unwrap();
                Ok(User {
                    uuid: user_uuid.to_owned(),
                    email: row.get(1).unwrap(),
                    name: row.get(2).unwrap(),
                    can_create: row.get(3).unwrap(),
                    is_me: current_user_uuid == user_uuid,
                })
            })
            .unwrap()
            .collect::<Result<Vec<User>, _>>()
        })
        .await
    }

    pub async fn gifts_of_list_user(
        self: &DbConn,
        list_uuid: String,
        user_uuid: String,
        current_user_uuid: String,
    ) -> Result<Vec<Gift>, Error> {
        self.run(move |db| {
            db.prepare(format!(
                "SELECT r.gift_uuid, r.user_uuid, r.url, r.comment, r.claimed, r.alternate_to, z.user_uuid, z.name \
                FROM {} l \
                INNER JOIN {} r ON r.gift_uuid = l.gift_uuid \
                LEFT JOIN {} z ON z.user_uuid = r.claimed_by \
                WHERE l.list_uuid = ?1 AND l.user_uuid = ?2",
                DbConn::LIST_TO_GIFT_TABLE,
                DbConn::GIFT_TABLE,
                DbConn::USER_TABLE,
            ).as_str())?
                .query_map(params![list_uuid, user_uuid],
                           |row| {
                               let user = match row.get::<usize, String>(6) {
                                   Ok(uuid) => Some(User {
                                       uuid: uuid.to_owned(),
                                       email: "".to_string(),
                                       name: row.get(7).unwrap(),
                                       can_create: false,
                                       is_me: uuid == current_user_uuid,
                                   }),
                                   Err(_) => None,
                               };
                               Ok(Gift {
                                   uuid: row.get(0).unwrap(),
                                   owner: row.get(1).unwrap(),
                                   url: row.get(2).unwrap(),
                                   comment: row.get(3).unwrap(),
                                   claimed: row.get(4).unwrap(),
                                   alternate_to_uuid: row.get(5).unwrap(),
                                   claimed_by: user,
                               })
                           }
                ).unwrap()
                .collect::<Result<Vec<Gift>, _>>()
        }).await
    }

    pub async fn gift_from_uuid(
        self: &DbConn,
        gift_uuid: String,
        current_user_uuid: String,
    ) -> Result<Gift, Error> {
        self.run(move |db| {
            db.query_row(format!(
                "SELECT l.gift_uuid, l.user_uuid, l.url, l.comment, l.claimed, l.alternate_to, r.user_uuid, r.name \
                FROM {} AS l \
                LEFT JOIN {} AS r ON r.user_uuid = l.claimed_by \
                WHERE l.gift_uuid = ?1",
                DbConn::GIFT_TABLE,
                DbConn::USER_TABLE,
            ).as_str(),
                         params![gift_uuid],
                         |row| {
                             let user = match row.get::<usize, String>(6) {
                                 Ok(uuid) => Some(User {
                                     uuid: uuid.to_owned(),
                                     email: "".to_string(),
                                     name: row.get(7).unwrap(),
                                     can_create: false,
                                     is_me: uuid == current_user_uuid,
                                 }),
                                 Err(_) => None,
                             };
                             Ok(Gift {
                                 uuid: row.get(0).unwrap(),
                                 owner: row.get(1).unwrap(),
                                 url: row.get(2).unwrap(),
                                 comment: row.get(3).unwrap(),
                                 claimed: row.get(4).unwrap(),
                                 alternate_to_uuid: row.get(5).unwrap(),
                                 claimed_by: user,
                             })
                         },
            )
        }).await
    }

    pub async fn create_user(self: &DbConn, user: User, auth_token: String) {
        self.run(move |db| {
            db.execute(
                format!(
                    "INSERT INTO {} (user_uuid, auth_token, email, name, can_create) \
                VALUES (?1, ?2, ?3, ?4, ?5)",
                    DbConn::USER_TABLE
                )
                .as_str(),
                params![
                    user.uuid,
                    auth_token,
                    user.email,
                    user.name,
                    user.can_create
                ],
            )
        })
        .await
        .unwrap();
    }

    pub async fn add_user_to_list(self: &DbConn, user_uuid: String, list_uuid: String) {
        self.run(move |db| {
            db.execute(
                format!(
                    "INSERT INTO {} (list_uuid, user_uuid) \
                VALUES (?1, ?2)",
                    DbConn::LIST_TO_USER_TABLE
                )
                .as_str(),
                params![list_uuid, user_uuid],
            )
        })
        .await
        .unwrap();
    }

    pub async fn create_list(self: &DbConn, list: List) {
        self.run(move |db| {
            db.execute(
                format!(
                    "INSERT INTO {} (list_uuid, name, owner) \
                VALUES (?1, ?2, ?3)",
                    DbConn::LIST_TABLE
                )
                .as_str(),
                params![list.uuid, list.name, list.owner],
            )
            .unwrap();
            db.execute(
                format!(
                    "INSERT INTO {} (list_uuid, user_uuid) \
                VALUES (?1, ?2)",
                    DbConn::LIST_TO_USER_TABLE
                )
                .as_str(),
                params![list.uuid, list.owner],
            )
            .unwrap();
        })
        .await;
    }

    pub async fn create_gift(self: &DbConn, gift: Gift, list_uuid: String) {
        self.run(move |db| {
            db.execute(format!(
                "INSERT INTO {} (gift_uuid, user_uuid, url, comment, claimed, claimed_by, alternate_to) \
                VALUES (?1, ?2, ?3, ?4, 0, 'None', ?5)",
                DbConn::GIFT_TABLE
            ).as_str(),
                       params![gift.uuid, gift.owner, gift.url, gift.comment, gift.alternate_to_uuid]
            ).unwrap();
            db.execute(format!(
                "INSERT INTO {} (gift_uuid, user_uuid, list_uuid) \
                VALUES (?1, ?2, ?3)",
                DbConn::LIST_TO_GIFT_TABLE
            ).as_str(),
                       params![gift.uuid, gift.owner, list_uuid]
            ).unwrap();
        }).await;
    }

    pub async fn modify_user(self: &DbConn, user: User) {
        self.run(move |db| {
            db.execute(
                format!(
                    "UPDATE {} SET name = ?1, email = ?2, can_crease = ?3 \
                WHERE user_uuid = ?4",
                    DbConn::USER_TABLE
                )
                .as_str(),
                params![user.name, user.email, user.can_create, user.uuid],
            )
        })
        .await
        .unwrap();
    }

    pub async fn modify_list(self: &DbConn, list: List) {
        self.run(move |db| {
            db.execute(
                format!(
                    "UPDATE {} SET name = ?1 \
                WHERE list_uuid = ?2",
                    DbConn::LIST_TABLE
                )
                .as_str(),
                params![list.name, list.uuid],
            )
        })
        .await
        .unwrap();
    }

    pub async fn modify_gift(self: &DbConn, gift: Gift) {
        self.run(move |db| {
            db.execute(
                format!(
                    "UPDATE {} SET url = ?1, comment = ?2, claimed = ?3, claimed_by = ?5 \
                WHERE gift_uuid = ?4",
                    DbConn::GIFT_TABLE
                )
                .as_str(),
                params![
                    gift.url,
                    gift.comment,
                    gift.claimed,
                    gift.uuid,
                    gift.claimed_by
                        .unwrap_or(User {
                            uuid: "None".to_string(),
                            email: "".to_string(),
                            name: "".to_string(),
                            can_create: false,
                            is_me: false,
                        })
                        .uuid
                ],
            )
        })
        .await
        .unwrap();
    }

    pub async fn delete_user(self: &DbConn, user_uuid: String) {
        // Unclaim items from user
        self.run(move |db| {
            db.execute(
                format!("DELETE FROM {} WHERE user_uuid = ?1", DbConn::USER_TABLE).as_str(),
                params![user_uuid],
            )
            .unwrap();
            db.execute(
                format!(
                    "DELETE FROM {} WHERE user_uuid = ?1",
                    DbConn::LIST_TO_USER_TABLE
                )
                .as_str(),
                params![user_uuid],
            )
            .unwrap();
            db.execute(
                format!(
                    "DELETE FROM {} l \
                INNER JOIN {} r ON r.gift_uuid = l.gift_uuid \
                INNER JOIN {} z ON z.list_uuid = r.list_uuid \
                WHERE z.owner = ?1",
                    DbConn::GIFT_TABLE,
                    DbConn::LIST_TO_GIFT_TABLE,
                    DbConn::LIST_TABLE
                )
                .as_str(),
                params![user_uuid],
            )
            .unwrap();
            db.execute(
                format!(
                    "DELETE FROM {} l \
                INNER JOIN {} r ON r.list_uuid = l.list_uuid \
                WHERE r.owner = ?1",
                    DbConn::LIST_TO_GIFT_TABLE,
                    DbConn::LIST_TABLE
                )
                .as_str(),
                params![user_uuid],
            )
            .unwrap();
            db.execute(
                format!("DELETE FROM {} WHERE owner = ?1", DbConn::LIST_TABLE).as_str(),
                params![user_uuid],
            )
            .unwrap();
            db.execute(
                format!("DELETE FROM {} WHERE user_uuid = ?1", DbConn::GIFT_TABLE).as_str(),
                params![user_uuid],
            )
            .unwrap();
        })
        .await;
    }

    pub async fn delete_list(self: &DbConn, list_uuid: String) {
        self.run(move |db| {
            db.execute(
                format!("DELETE FROM {} WHERE list_uuid = ?1", DbConn::LIST_TABLE).as_str(),
                params![list_uuid],
            )
            .unwrap();
            db.execute(
                format!(
                    "DELETE FROM {} WHERE list_uuid = ?1",
                    DbConn::LIST_TO_USER_TABLE
                )
                .as_str(),
                params![list_uuid],
            )
            .unwrap();
            db.execute(
                format!(
                    "DELETE FROM {} l \
                INNER JOIN {} r ON r.gift_uuid = l.gift_uuid \
                WHERE l.list_uuid = ?1",
                    DbConn::GIFT_TABLE,
                    DbConn::LIST_TO_GIFT_TABLE
                )
                .as_str(),
                params![list_uuid],
            )
            .unwrap();
            db.execute(
                format!(
                    "DELETE FROM {} WHERE list_uuid = ?1",
                    DbConn::LIST_TO_GIFT_TABLE
                )
                .as_str(),
                params![list_uuid],
            )
            .unwrap();
        })
        .await;
    }

    pub async fn delete_gift(self: &DbConn, gift_uuid: String) {
        self.run(move |db| {
            db.execute(
                format!(
                    "DELETE FROM {} WHERE gift_uuid = ?1",
                    DbConn::LIST_TO_GIFT_TABLE
                )
                .as_str(),
                params![gift_uuid],
            )
            .unwrap();
            db.execute(
                format!("DELETE FROM {} WHERE gift_uuid = ?1", DbConn::GIFT_TABLE).as_str(),
                params![gift_uuid],
            )
            .unwrap();
        })
        .await;
    }

    pub async fn init_db(rocket: Rocket<Build>) -> Rocket<Build> {
        let conn = DbConn::get_one(&rocket).await.expect("database mounted");

        conn.run(move |db| {
            db.execute(
                format!(
                    "CREATE TABLE IF NOT EXISTS {} (
                        user_uuid   TEXT PRIMARY KEY,
                        auth_token  TEXT NOT NULL,
                        email       TEXT NOT NULL,
                        name        TEXT NOT NULL,
                        can_create  BOOL NOT NULL
                        )",
                    DbConn::USER_TABLE
                )
                .as_str(),
                [],
            )
        })
        .await
        .unwrap();

        conn.run(move |db| {
            db.execute(
                format!(
                    "CREATE INDEX IF NOT EXISTS tokens ON {}(auth_token)",
                    DbConn::USER_TABLE
                )
                .as_str(),
                [],
            )
        })
        .await
        .unwrap();

        conn.run(move |db| {
            db.execute(
                format!(
                    "CREATE INDEX IF NOT EXISTS emails ON {}(email)",
                    DbConn::USER_TABLE
                )
                .as_str(),
                [],
            )
        })
        .await
        .unwrap();

        conn.run(move |db| {
            db.execute(
                format!(
                    "CREATE TABLE IF NOT EXISTS {} (
                        list_uuid   TEXT PRIMARY KEY,
                        name        TEXT NOT NULL,
                        owner       TEXT NOT NULL
                        )",
                    DbConn::LIST_TABLE
                )
                .as_str(),
                [],
            )
        })
        .await
        .unwrap();

        conn.run(move |db| {
            db.execute(
                format!(
                    "CREATE TABLE IF NOT EXISTS {} (
                        list_uuid   TEXT,
                        user_uuid   TEXT
                        )",
                    DbConn::LIST_TO_USER_TABLE
                )
                .as_str(),
                [],
            )
        })
        .await
        .unwrap();

        conn.run(move |db| {
            db.execute(
                format!(
                    "CREATE INDEX IF NOT EXISTS list_map_list ON {}(list_uuid)",
                    DbConn::LIST_TO_USER_TABLE
                )
                .as_str(),
                [],
            )
        })
        .await
        .unwrap();

        conn.run(move |db| {
            db.execute(
                format!(
                    "CREATE INDEX IF NOT EXISTS list_map_user ON {}(user_uuid)",
                    DbConn::LIST_TO_USER_TABLE
                )
                .as_str(),
                [],
            )
        })
        .await
        .unwrap();

        conn.run(move |db| {
            db.execute(
                format!(
                    "CREATE TABLE IF NOT EXISTS {} (
                        gift_uuid       TEXT PRIMARY KEY,
                        user_uuid       TEXT NOT NULL,
                        url             TEXT NOT NULL,
                        comment         TEXT NOT NULL,
                        claimed         BOOL NOT NULL,
                        claimed_by      TEXT NOT NULL,
                        alternate_to    TEXT
                        )",
                    DbConn::GIFT_TABLE
                )
                .as_str(),
                [],
            )
        })
        .await
        .unwrap();

        conn.run(move |db| {
            db.execute(
                format!(
                    "CREATE TABLE IF NOT EXISTS {} (
                        list_uuid   TEXT,
                        user_uuid   TEXT,
                        gift_uuid   TEXT
                        )",
                    DbConn::LIST_TO_GIFT_TABLE
                )
                .as_str(),
                [],
            )
        })
        .await
        .unwrap();

        conn.run(move |db| {
            db.execute(
                format!(
                    "CREATE INDEX IF NOT EXISTS item_map_list ON {}(list_uuid)",
                    DbConn::LIST_TO_GIFT_TABLE
                )
                .as_str(),
                [],
            )
        })
        .await
        .unwrap();

        rocket
    }
}
