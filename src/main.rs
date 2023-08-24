#[macro_use]
extern crate rocket;

mod data;
mod db;
mod email;
mod webserver;

use rocket::fairing::AdHoc;
use rocket::form::Form;
use rocket::fs::{relative, FileServer};
use rocket::http::{CookieJar, Status};
use rocket::request::{FromRequest, Outcome, Request};
use rocket::response::Redirect;
use rocket_dyn_templates::Template;

use crate::data::{Auth, ClaimGift, CreateList, DeleteList, ModifyGiftList, ModifyList};
use crate::db::DbConn;
use crate::webserver::WebServer;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Auth {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> Outcome<Auth, Self::Error> {
        let user_uuid = WebServer::get_user_cookie(req.cookies());
        return match user_uuid {
            Ok(_) => Outcome::Success(Auth {
                user_token: "".to_string(),
            }),
            Err(_) => Outcome::Failure((Status::NotFound, ())),
        };
    }
}

#[catch(default)]
async fn default_error() -> Template {
    WebServer::not_found().await
}

#[catch(403)]
async fn access_denied() -> Template {
    WebServer::access_denied().await
}

#[catch(500)]
async fn internal_error() -> Template {
    WebServer::internal_error().await
}

#[get("/")]
async fn index() -> Template {
    WebServer::index().await
}

#[get("/login/<usertoken>")]
async fn login(usertoken: String, cookies: &CookieJar<'_>, conn: DbConn) -> Redirect {
    WebServer::login(usertoken, cookies, &conn).await
}

#[get("/user")]
async fn user_page(cookies: &CookieJar<'_>, conn: DbConn, _auth: Auth) -> Template {
    WebServer::user_page(cookies, &conn).await
}

#[get("/list/<list_uuid>")]
async fn list_page(
    list_uuid: String,
    cookies: &CookieJar<'_>,
    conn: DbConn,
    _auth: Auth,
) -> Template {
    WebServer::list_page(list_uuid, cookies, &conn).await
}

#[get("/list/<list_uuid>/<user_uuid>")]
async fn list_user_page(
    list_uuid: String,
    user_uuid: String,
    cookies: &CookieJar<'_>,
    conn: DbConn,
    _auth: Auth,
) -> Template {
    WebServer::list_user_page(list_uuid, user_uuid, cookies, &conn).await
}

#[get("/createlist")]
async fn create_list_page(cookies: &CookieJar<'_>, conn: DbConn, _auth: Auth) -> Template {
    WebServer::create_list_page(cookies, &conn).await
}

#[post("/createlist", data = "<list>")]
async fn create_list(
    list: Form<CreateList>,
    cookies: &CookieJar<'_>,
    conn: DbConn,
    _auth: Auth,
) -> String {
    WebServer::create_list(list, cookies, &conn).await
}

#[get("/modifylist/<list_uuid>")]
async fn modify_list_page(
    list_uuid: String,
    cookies: &CookieJar<'_>,
    conn: DbConn,
    _auth: Auth,
) -> Template {
    WebServer::modify_list_page(list_uuid, cookies, &conn).await
}

#[post("/modifylist", data = "<list>")]
async fn modify_list(
    list: Form<ModifyList>,
    cookies: &CookieJar<'_>,
    conn: DbConn,
    _auth: Auth,
) -> String {
    WebServer::modify_list(list, cookies, &conn).await
}

#[post("/deletelist", data = "<list>")]
async fn delete_list(
    list: Form<DeleteList>,
    cookies: &CookieJar<'_>,
    conn: DbConn,
    _auth: Auth,
) -> String {
    WebServer::delete_list(list, cookies, &conn).await
}

#[post("/claim", data = "<claim>")]
async fn claim_gift(
    claim: Form<ClaimGift>,
    cookies: &CookieJar<'_>,
    conn: DbConn,
    _auth: Auth,
) -> String {
    WebServer::claim_gift(claim, cookies, &conn).await
}

#[post("/unclaim", data = "<claim>")]
async fn unclaim_gift(
    claim: Form<ClaimGift>,
    cookies: &CookieJar<'_>,
    conn: DbConn,
    _auth: Auth,
) -> String {
    WebServer::unclaim_gift(claim, cookies, &conn).await
}

#[post("/modifygiftlist", data = "<gifts>")]
async fn modify_item_list(
    gifts: Form<ModifyGiftList>,
    cookies: &CookieJar<'_>,
    conn: DbConn,
    _auth: Auth,
) -> String {
    WebServer::modify_gift_list(gifts, cookies, &conn).await
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount(
            "/",
            routes![
                index,
                login,
                user_page,
                list_page,
                list_user_page,
                create_list_page,
                create_list,
                modify_list_page,
                modify_list,
                delete_list,
                claim_gift,
                unclaim_gift,
                modify_item_list,
            ],
        )
        .mount("/", FileServer::from(relative!("static")))
        .register("/", catchers![internal_error, access_denied, default_error])
        .attach(Template::fairing())
        .attach(DbConn::fairing())
        .attach(AdHoc::on_ignite("Rusqlite Init", DbConn::init_db))
}
