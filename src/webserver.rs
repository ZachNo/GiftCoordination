use rand::{rngs::StdRng, RngCore, SeedableRng};
use rocket::form::Form;
use rocket::http::{Cookie, CookieJar, SameSite};
use rocket::response::Redirect;
use rocket_dyn_templates::Template;
use serde::Serialize;
use serde_json;
use std::collections::HashMap;
use uuid::Uuid;

use crate::data::{
    ClaimGift, CreateList, DeleteList, ExportGift, Gift, List, ListPage, ListUser, ListUserPage,
    ModifyGiftList, ModifyList, ModifyListPage, UnsubscribePage, User, UserPage,
};
use crate::db::DbConn;
use crate::email::Email;

pub struct WebServer;

impl WebServer {
    const USER_COOKIE: &'static str = "userToken";

    pub async fn access_denied() -> Template {
        Template::render("access_denied", "")
    }

    pub async fn not_found() -> Template {
        Template::render("not_found", "")
    }

    pub async fn internal_error() -> Template {
        Template::render("internal_error", "")
    }

    pub async fn index() -> Template {
        Template::render("index", "")
    }

    pub async fn login(user_token: String, cookies: &CookieJar<'_>, conn: &DbConn) -> Redirect {
        let user_uuid = conn.user_uuid_from_auth_token(user_token).await;
        return match user_uuid {
            Ok(u) => {
                WebServer::set_user_cookie(u, cookies);
                Redirect::to(uri!("/user"))
            }
            Err(_) => Redirect::to(uri!("/")),
        };
    }

    pub async fn user_page(cookies: &CookieJar<'_>, conn: &DbConn) -> Template {
        let current_user = WebServer::get_current_user(cookies, conn).await.unwrap();
        let lists = conn
            .lists_of_user(current_user.uuid.to_owned())
            .await
            .unwrap();
        let context = UserPage {
            current_user,
            lists,
        };
        Template::render("user", &context)
    }

    /// GET only shows a confirmation, so mail scanners that prefetch links can't unsubscribe anyone.
    pub async fn unsubscribe_page(user_token: String, conn: &DbConn) -> Template {
        if conn
            .user_uuid_from_auth_token(user_token.to_owned())
            .await
            .is_err()
        {
            return WebServer::not_found().await;
        }
        Template::render(
            "unsubscribe",
            &UnsubscribePage {
                token: user_token,
                done: false,
            },
        )
    }

    pub async fn unsubscribe(user_token: String, conn: &DbConn) -> Template {
        let user_uuid = match conn.user_uuid_from_auth_token(user_token.to_owned()).await {
            Ok(u) => u,
            Err(_) => return WebServer::not_found().await,
        };
        let mut user = conn
            .user_from_uuid(user_uuid.to_owned(), user_uuid)
            .await
            .unwrap();
        user.do_email = false;
        conn.modify_user(user).await;
        Template::render(
            "unsubscribe",
            &UnsubscribePage {
                token: user_token,
                done: true,
            },
        )
    }

    pub async fn list_page(list_uuid: String, cookies: &CookieJar<'_>, conn: &DbConn) -> Template {
        let current_user = WebServer::get_current_user(cookies, conn).await.unwrap();
        if !conn
            .is_user_in_list(current_user.uuid.to_owned(), list_uuid.to_owned())
            .await
        {
            return WebServer::not_found().await;
        }
        let users = conn
            .users_of_list(list_uuid.to_owned(), current_user.uuid.to_owned())
            .await
            .unwrap();
        let mut list = conn
            .list_from_uuid(list_uuid, current_user.uuid.to_owned())
            .await
            .unwrap();
        let owner_name = conn
            .user_from_uuid(list.owner, current_user.uuid.to_owned())
            .await
            .unwrap();
        list.owner = owner_name.name;
        let context = ListPage {
            list,
            current_user,
            users,
        };
        Template::render("list", &context)
    }

    pub async fn list_user_page(
        list_uuid: String,
        user_uuid: String,
        cookies: &CookieJar<'_>,
        conn: &DbConn,
    ) -> Template {
        let current_user = WebServer::get_current_user(cookies, conn).await.unwrap();
        // Both the viewer and the viewed user must belong to this list
        if !conn
            .is_user_in_list(current_user.uuid.to_owned(), list_uuid.to_owned())
            .await
            || !conn
                .is_user_in_list(user_uuid.to_owned(), list_uuid.to_owned())
                .await
        {
            return WebServer::not_found().await;
        }
        let requested_user = conn
            .user_from_uuid(user_uuid.to_owned(), current_user.uuid.to_owned())
            .await
            .unwrap();
        let is_me = requested_user.is_me;
        let list = conn
            .list_from_uuid(list_uuid.to_owned(), current_user.uuid.to_owned())
            .await
            .unwrap();
        let gifts = conn
            .gifts_of_list_user(
                list_uuid,
                user_uuid.to_owned(),
                current_user.uuid.to_owned(),
            )
            .await
            .unwrap();
        let gifts_export = gifts
            .iter()
            .map(|gift| {
                // Never send claim details to the gift's owner; it would spoil the surprise
                let (claimed, claimed_by_name, claimed_by_me) = if is_me {
                    (false, "".to_string(), false)
                } else {
                    match &gift.claimed_by {
                        Some(x) => (gift.claimed, x.name.to_owned(), x.is_me),
                        None => (gift.claimed, "".to_string(), false),
                    }
                };
                ExportGift {
                    uuid: gift.uuid.to_owned(),
                    url: gift.url.to_owned(),
                    comment: gift.comment.to_owned(),
                    claimed,
                    claimed_by_name,
                    claimed_by_me,
                    alternate_to_uuid: gift.alternate_to_uuid.to_owned(),
                }
            })
            .collect::<Vec<_>>();
        let gifts_json = WebServer::json_for_script(&gifts_export);

        let context = ListUserPage {
            user: requested_user,
            current_user,
            gifts_data: gifts_json,
            list,
        };

        if is_me {
            Template::render("list_user_self", &context)
        } else {
            Template::render("list_user_other", &context)
        }
    }

    pub async fn create_list_page(cookies: &CookieJar<'_>, conn: &DbConn) -> Template {
        let current_user = WebServer::get_current_user(cookies, conn).await.unwrap();
        if !current_user.can_create {
            return WebServer::not_found().await;
        }
        Template::render("create_list", "")
    }

    pub async fn modify_list_page(
        list_uuid: String,
        cookies: &CookieJar<'_>,
        conn: &DbConn,
    ) -> Template {
        let current_user = WebServer::get_current_user(cookies, conn).await.unwrap();
        let list = conn
            .list_from_uuid(list_uuid, current_user.uuid.to_owned())
            .await
            .unwrap();
        if list.owner != current_user.uuid {
            return WebServer::not_found().await;
        }
        let list_users = conn
            .users_of_list(list.uuid.to_owned(), current_user.uuid.to_owned())
            .await
            .unwrap();
        let users = list_users
            .iter()
            .map(|u| ListUser {
                name: u.name.to_owned(),
                email: u.email.to_owned(),
            })
            .collect();
        let context = ModifyListPage {
            current_user,
            list,
            users,
        };
        Template::render("modify_list", &context)
    }

    pub async fn claim_gift(
        claim: Form<ClaimGift>,
        cookies: &CookieJar<'_>,
        conn: &DbConn,
    ) -> String {
        let current_user = WebServer::get_current_user(cookies, conn).await.unwrap();
        if !conn
            .can_user_see_gift(current_user.uuid.to_owned(), claim.gift_uuid.to_owned())
            .await
        {
            return "Item not found".to_string();
        }
        let gift = conn
            .gift_from_uuid(claim.gift_uuid.to_owned(), current_user.uuid.to_owned())
            .await
            .unwrap();
        if gift.owner == current_user.uuid {
            return "You can't claim your own gifts :|".to_string();
        }
        if gift.claimed {
            return format!("Item already claimed by {}", gift.claimed_by.unwrap().name);
        }
        if !conn
            .try_claim_gift(gift.uuid, current_user.uuid.to_owned())
            .await
        {
            return "Someone else just claimed this item".to_string();
        }

        "Claimed!".to_string()
    }

    pub async fn unclaim_gift(
        claim: Form<ClaimGift>,
        cookies: &CookieJar<'_>,
        conn: &DbConn,
    ) -> String {
        let current_user = WebServer::get_current_user(cookies, conn).await.unwrap();
        if !conn
            .can_user_see_gift(current_user.uuid.to_owned(), claim.gift_uuid.to_owned())
            .await
        {
            return "Item not found".to_string();
        }
        let gift = conn
            .gift_from_uuid(claim.gift_uuid.to_owned(), current_user.uuid.to_owned())
            .await
            .unwrap();
        if gift.owner == current_user.uuid.to_owned() {
            return "You can't unclaim your own gifts :|".to_string();
        }
        if !gift.claimed {
            return "Item isn't claimed".to_string();
        }
        if gift.claimed_by.as_ref().unwrap().uuid != current_user.uuid {
            return format!("Item claimed by {}", gift.claimed_by.unwrap().name);
        }

        if !conn
            .try_unclaim_gift(gift.uuid, current_user.uuid.to_owned())
            .await
        {
            return "Item isn't claimed by you".to_string();
        }

        "Unclaimed!".to_string()
    }

    pub async fn modify_gift_list(
        gifts: Form<ModifyGiftList>,
        cookies: &CookieJar<'_>,
        conn: &DbConn,
    ) -> String {
        let current_user = WebServer::get_current_user(cookies, conn).await.unwrap();
        if !conn
            .is_user_in_list(current_user.uuid.to_owned(), gifts.list_uuid.to_owned())
            .await
        {
            return "Unauthorized".to_string();
        }

        let mut new_uuid_map: HashMap<String, String> = HashMap::new();
        let existing_gifts = conn
            .gifts_of_list_user(
                gifts.list_uuid.to_owned(),
                current_user.uuid.to_owned(),
                current_user.uuid.to_owned(),
            )
            .await
            .unwrap();
        let existing_gifts_uuids = existing_gifts
            .iter()
            .map(|gift| gift.uuid.to_owned())
            .collect::<Vec<_>>();
        let new_gifts_uuid = gifts
            .gifts
            .iter()
            .map(|gift| {
                let mut uuid = gift.uuid.to_owned();
                if uuid.as_str().starts_with("newRow-") {
                    new_uuid_map.insert(uuid.to_owned(), Uuid::new_v4().to_string());
                    uuid = "".to_string();
                }
                uuid
            })
            .collect::<Vec<_>>();

        for old_gift_uuid in existing_gifts_uuids {
            if !new_gifts_uuid.contains(&old_gift_uuid) {
                conn.delete_gift(old_gift_uuid).await;
            }
        }

        for gift in &gifts.gifts {
            let alternate = gift.alternate_to_uuid.to_owned();
            let opt_alt = if alternate.starts_with("newRow-") {
                Option::Some(new_uuid_map.get(alternate.as_str()).unwrap().to_owned())
            } else if alternate.is_empty() {
                Option::None
            } else {
                Option::Some(alternate)
            };
            if gift.uuid.is_empty() {
                let gift_data = Gift {
                    uuid: Uuid::new_v4().to_string(),
                    owner: current_user.uuid.to_owned(),
                    url: gift.url.to_owned(),
                    comment: gift.comment.to_owned(),
                    claimed: false,
                    claimed_by: None,
                    alternate_to_uuid: opt_alt,
                };
                conn.create_gift(gift_data, gifts.list_uuid.to_owned())
                    .await;
            } else if gift.uuid.starts_with("newRow-") {
                let gift_data = Gift {
                    uuid: new_uuid_map.get(gift.uuid.as_str()).unwrap().to_owned(),
                    owner: current_user.uuid.to_owned(),
                    url: gift.url.to_owned(),
                    comment: gift.comment.to_owned(),
                    claimed: false,
                    claimed_by: None,
                    alternate_to_uuid: opt_alt,
                };
                conn.create_gift(gift_data, gifts.list_uuid.to_owned())
                    .await;
            } else {
                let gift_data = conn
                    .gift_from_uuid(gift.uuid.to_owned(), current_user.uuid.to_owned())
                    .await
                    .unwrap();
                if gift_data.owner != current_user.uuid {
                    return "Can't modify item you don't own".to_string();
                }

                conn.update_gift_details(
                    gift_data.uuid,
                    gift.url.to_owned(),
                    gift.comment.to_owned(),
                )
                .await;
            }
        }
        "Success!".to_string()
    }

    pub async fn create_list(
        list: Form<CreateList>,
        cookies: &CookieJar<'_>,
        conn: &DbConn,
    ) -> String {
        let current_user = WebServer::get_current_user(cookies, conn).await.unwrap();
        if !current_user.can_create {
            return "You don't have permission to create lists".to_string();
        }
        let list_uuid = Uuid::new_v4().to_string();
        let list_data = List {
            uuid: list_uuid.to_owned(),
            name: list.name.to_owned(),
            owner: current_user.uuid.to_owned(),
            im_owner: true,
        };
        conn.create_list(list_data).await;
        for list_user in list.users.to_owned() {
            if list_user.name.is_empty() || list_user.email.is_empty() {
                continue;
            }
            WebServer::create_user_and_add_to_list(
                list.name.to_owned(),
                list_uuid.to_owned(),
                current_user.uuid.to_owned(),
                list_user,
                conn,
            )
            .await;
        }
        "Success!".to_string()
    }

    pub async fn modify_list(
        list: Form<ModifyList>,
        cookies: &CookieJar<'_>,
        conn: &DbConn,
    ) -> String {
        let current_user = WebServer::get_current_user(cookies, conn).await.unwrap();
        let mut list_data = conn
            .list_from_uuid(list.uuid.to_owned(), current_user.uuid.to_owned())
            .await
            .unwrap();
        if !list_data.im_owner {
            return "Can't modify list you don't own".to_string();
        }
        list_data.name = list.name.to_owned();
        conn.modify_list(list_data).await;
        let users = conn
            .users_of_list(list.uuid.to_owned(), current_user.uuid.to_owned())
            .await
            .unwrap();
        let existing_user_emails: Vec<String> = users.iter().map(|u| u.email.to_owned()).collect();
        for list_user in list.users.to_owned() {
            if existing_user_emails.contains(&list_user.email) {
                continue;
            }
            WebServer::create_user_and_add_to_list(
                list.name.to_owned(),
                list.uuid.to_owned(),
                current_user.uuid.to_owned(),
                list_user,
                conn,
            )
            .await;
        }
        // TODO: Remove deleted users
        "Success!".to_string()
    }

    pub async fn delete_list(
        list: Form<DeleteList>,
        cookies: &CookieJar<'_>,
        conn: &DbConn,
    ) -> String {
        // Check to see if we should warn about deleting
        let current_user = WebServer::get_current_user(cookies, conn).await.unwrap();
        let list_data = conn
            .list_from_uuid(list.uuid.to_owned(), current_user.uuid.to_owned())
            .await
            .unwrap();
        if !list_data.im_owner {
            return "Can't delete list you don't own".to_string();
        }
        conn.delete_list(list.uuid.to_owned()).await;
        "Success!".to_string()
    }

    async fn send_email_for_list(list_name: String, user: User, auth_token: String) {
        let email_client = Email::build();
        email_client
            .send_invite_email(list_name, auth_token, user.name, user.email)
            .unwrap();
    }

    async fn create_user_and_add_to_list(
        list_name: String,
        list_uuid: String,
        current_user_uuid: String,
        list_user: ListUser,
        conn: &DbConn,
    ) {
        let user = match conn.user_from_email(list_user.email.to_owned()).await {
            Ok(u) => u,
            Err(_) => {
                let mut rng = StdRng::from_os_rng();
                let mut new_token: [u8; 256] = [0; 256];
                rng.fill_bytes(&mut new_token);
                let new_user_uuid = Uuid::new_v4().to_string();
                let new_user = User {
                    uuid: new_user_uuid.to_owned(),
                    email: list_user.email.to_owned(),
                    name: list_user.name.to_owned(),
                    can_create: false,
                    is_me: false,
                    do_email: true,
                };
                conn.create_user(new_user, hex::encode(new_token)).await;
                conn.user_from_uuid(new_user_uuid, current_user_uuid)
                    .await
                    .unwrap()
            }
        };
        conn.add_user_to_list(user.uuid.to_owned(), list_uuid).await;
        let auth_token = conn
            .auth_token_from_user(user.uuid.to_owned())
            .await
            .unwrap();
        if user.do_email {
            WebServer::send_email_for_list(list_name, user, auth_token).await;
        }
    }

    /// Serializes data for embedding inside a `<script>` block. Escaping `<`, `>` and `&`
    /// keeps user text like `</script>` from closing the block early.
    fn json_for_script<T: Serialize>(value: &T) -> String {
        serde_json::to_string(value)
            .unwrap()
            .replace('<', "\\u003c")
            .replace('>', "\\u003e")
            .replace('&', "\\u0026")
    }

    fn set_user_cookie(user_uuid: String, cookies: &CookieJar<'_>) {
        let mut cookie = Cookie::new(WebServer::USER_COOKIE, user_uuid);
        cookie.set_same_site(SameSite::Lax);
        cookies.add_private(cookie);
    }

    pub fn get_user_cookie(cookies: &CookieJar<'_>) -> Result<String, String> {
        let cookie = cookies.get_private(WebServer::USER_COOKIE);
        return match cookie {
            Some(c) => Ok(c.value().to_string()),
            None => Err("No cookie retrieved".to_string()),
        };
    }

    async fn get_current_user(cookies: &CookieJar<'_>, conn: &DbConn) -> Result<User, String> {
        let current_user_uuid = WebServer::get_user_cookie(cookies);
        return match current_user_uuid {
            Ok(u) => match conn.user_from_uuid(u.to_owned(), u).await {
                Ok(r) => Ok(r),
                Err(e) => Err(e.to_string()),
            },
            Err(e) => Err(e),
        };
    }
}
