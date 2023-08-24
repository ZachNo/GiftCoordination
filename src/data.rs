use serde::Serialize;

// Page Data
#[derive(Serialize)]
pub struct User {
    pub uuid: String,
    pub email: String,
    pub name: String,
    pub can_create: bool,
    pub is_me: bool,
}

#[derive(Serialize)]
pub struct List {
    pub uuid: String,
    pub name: String,
    pub owner: String,
    pub im_owner: bool,
}

#[derive(Serialize)]
pub struct Gift {
    pub uuid: String,
    pub owner: String,
    pub url: String,
    pub comment: String,
    pub claimed: bool,
    pub claimed_by: Option<User>,
    pub alternate_to_uuid: Option<String>,
}

#[derive(Serialize)]
pub struct ExportGift {
    pub uuid: String,
    pub url: String,
    pub comment: String,
    pub claimed: bool,
    pub claimed_by_name: String,
    pub claimed_by_me: bool,
    pub alternate_to_uuid: Option<String>,
}

// Email
#[derive(Serialize)]
pub struct InviteEmail {
    pub user_name: String,
    pub list_name: String,
    pub link: String,
    pub admin_email: String,
}

// Page Contexts
#[derive(Serialize)]
pub struct UserPage {
    pub current_user: User,
    pub lists: Vec<List>,
}

#[derive(Serialize)]
pub struct ListPage {
    pub list: List,
    pub current_user: User,
    pub users: Vec<User>,
}

#[derive(Serialize)]
pub struct ListUserPage {
    pub user: User,
    pub current_user: User,
    pub list: List,
    pub gifts_data: String,
}

#[derive(Serialize)]
pub struct ModifyListPage {
    pub current_user: User,
    pub list: List,
    pub users: Vec<ListUser>,
}

// Page Input
#[derive(Clone, FromForm, Serialize)]
pub struct ListUser {
    pub name: String,
    pub email: String,
}

#[derive(FromForm)]
pub struct CreateList {
    pub name: String,
    pub users: Vec<ListUser>,
}

#[derive(FromForm)]
pub struct ModifyList {
    pub uuid: String,
    pub name: String,
    pub users: Vec<ListUser>,
}

#[derive(FromForm)]
pub struct DeleteList {
    pub uuid: String,
}

#[derive(FromForm)]
pub struct ClaimGift {
    pub gift_uuid: String,
}

#[derive(Clone, FromForm, Serialize)]
pub struct FormGift {
    pub uuid: String,
    pub url: String,
    pub comment: String,
    pub alternate_to_uuid: String,
}

#[derive(FromForm)]
pub struct ModifyGiftList {
    pub list_uuid: String,
    pub gifts: Vec<FormGift>,
}

// Page Auth
#[derive(FromForm)]
pub struct Auth {
    pub user_token: String,
}
