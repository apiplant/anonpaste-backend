use sendgrid::v3::{
    ClickTrackingSetting, Content, Email, Message, OpenTrackingSetting, Personalization, Sender,
    SubscriptionTrackingSetting, TrackingSettings,
};

#[derive(Debug, Clone)]
pub struct Mailer {
    sendgrid_api_key: String,
    from_email: String,
    from_name: String,
}

impl Mailer {
    pub fn new(sendgrid_api_key: String, from_email: String, from_name: String) -> Self {
        Mailer {
            sendgrid_api_key,
            from_email,
            from_name,
        }
    }

    pub async fn respond_to(&self, to_email: &str, links: &Vec<String>) -> anyhow::Result<()> {
        let from = Email::new(&self.from_email).set_name(&self.from_name);
        let to = Email::new(to_email);
        let subject = "DMCA Report Initiated";
        let t = TrackingSettings {
            click_tracking: Some(ClickTrackingSetting {
                enable: Some(false),
                enable_text: None,
            }),
            open_tracking: Some(OpenTrackingSetting {
                enable: Some(false),
                substitution_tag: None,
            }),
            subscription_tracking: Some(SubscriptionTrackingSetting {
                enable: Some(false),
            }),
        };
        let message = Message::new(from.clone())
            .set_subject(subject)
            .set_tracking_settings(t.clone())
            .add_content(
                Content::new().set_content_type("text/plain").set_value(
                    vec![
                        "Thanks for reaching out and initiating our DMCA Report procedure.",
                        "You reported the following links:\n",
                        &links.join("\n"),
                        "\nAn operator will get back to you within 24hrs.\n",
                        "Kind Regards,",
                        "AnonPaste Team",
                    ]
                    .join("\n"),
                ),
            )
            .add_personalization(Personalization::new(to.clone()));

        Sender::new(self.sendgrid_api_key.clone())
            .send(&message)
            .await?;

        let message = Message::new(from.clone())
            .set_subject(subject)
            .set_tracking_settings(t)
            .add_content(
                Content::new()
                    .set_content_type("text/plain")
                    .set_value(format!(
                        "These links have been reported by {}:\n\n{}\n\nAnonPaste Team",
                        to_email,
                        links.join("\n")
                    )),
            )
            .add_personalization(Personalization::new(from));

        Sender::new(self.sendgrid_api_key.clone())
            .send(&message)
            .await?;

        Ok(())
    }
}
