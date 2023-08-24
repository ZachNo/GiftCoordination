use crate::data::InviteEmail;
use config_file::FromConfigFile;
use lettre::transport::smtp::response::Response;
use lettre::transport::smtp::Error;
use lettre::{
    message::{header, Mailbox, MultiPart, SinglePart},
    transport::smtp::authentication::Credentials,
    Message, SmtpTransport, Transport,
};
use rocket_dyn_templates::handlebars::Handlebars;
use serde::Deserialize;

pub struct Email {
    creds: Credentials,
    email_endpoint: String,
    email_from: String,
    admin_email: String,
    website_root: String,
}

#[derive(Deserialize)]
struct EmailConfig {
    access_key: String,
    secret_key: String,
    email_endpoint: String,
    email_from: String,
    admin_email: String,
    website_root: String,
}

impl Email {
    pub fn build() -> Email {
        let config = EmailConfig::from_config_file("config.toml").unwrap();
        let creds = Credentials::new(config.access_key, config.secret_key);
        Email {
            creds,
            email_endpoint: config.email_endpoint,
            email_from: config.email_from,
            admin_email: config.admin_email,
            website_root: config.website_root,
        }
    }

    fn create_relay(self: Email) -> SmtpTransport {
        SmtpTransport::starttls_relay(self.email_endpoint.as_str())
            .unwrap()
            .credentials(self.creds)
            .build()
    }

    pub fn send_invite_email(
        self: Email,
        list_name: String,
        user_auth_token: String,
        user_name: String,
        user_email: String,
    ) -> Result<Response, Error> {
        let img = std::fs::read("./static/header.png").unwrap();
        let mut handlebars = Handlebars::new();
        handlebars
            .register_template_file("email_invite", "./templates/email_invite.html.hbs")
            .unwrap();
        let emailed_from: Mailbox = self.email_from.parse().unwrap();
        let emailed_to: Mailbox = format!("{} <{}>", user_name, user_email).parse().unwrap();
        let email = Message::builder()
            .from(emailed_from)
            .to(emailed_to)
            .subject(format!(
                "You've been invited to the {} wishlist!",
                list_name
            ))
            .multipart(
                MultiPart::related()
                    .singlepart(
                        SinglePart::builder()
                            .header(header::ContentType::TEXT_HTML)
                            .body(
                                handlebars
                                    .render(
                                        "email_invite",
                                        &InviteEmail {
                                            user_name,
                                            list_name,
                                            link: format!(
                                                "{}login/{}",
                                                self.website_root.as_str(),
                                                user_auth_token
                                            ),
                                            admin_email: self.admin_email.to_string(),
                                        },
                                    )
                                    .unwrap(),
                            ),
                    )
                    .singlepart(
                        SinglePart::builder()
                            .header(header::ContentType::parse("image/png").unwrap())
                            .header(header::ContentDisposition::inline())
                            .header(header::ContentId::from(String::from("<123>")))
                            .body(img),
                    ),
            )
            .unwrap();
        let mailer = self.create_relay();
        mailer.send(&email)
    }
}
