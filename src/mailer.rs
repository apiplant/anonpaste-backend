use sendgrid::v3::{
    ClickTrackingSetting, Content, Email, Message, OpenTrackingSetting, Personalization, Sender,
    SubscriptionTrackingSetting, TrackingSettings,
};
use std::sync::{Arc, Mutex};

pub struct Mailer {
    sendgrid_api_key: String,
    email_from: String,
    email_name: String,
    sent: Arc<Mutex<Vec<ReportMessage>>>,
}

impl Clone for Mailer {
    fn clone(&self) -> Self {
        Mailer {
            sendgrid_api_key: self.sendgrid_api_key.clone(),
            email_from: self.email_from.clone(),
            email_name: self.email_name.clone(),
            sent: self.sent.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportMessage {
    pub email_from: String,
    pub email_name: String,
    pub to: String,
    pub content: String,
    pub subject: String,
}

impl Into<Message> for ReportMessage {
    fn into(self) -> Message {
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

        Message::new(Email::new(&self.email_from).set_name(&self.email_name))
            .set_subject(&self.subject)
            .set_tracking_settings(t)
            .add_content(
                Content::new()
                    .set_content_type("text/plain")
                    .set_value(self.content),
            )
            .add_personalization(Personalization::new(Email::new(&self.to)))
    }
}

impl Mailer {
    pub fn new(sendgrid_api_key: String, email_from: String, email_name: String) -> Self {
        Mailer {
            sendgrid_api_key,
            email_from,
            email_name,
            sent: Arc::new(Mutex::from(Vec::new())),
        }
    }

    pub async fn respond_to(&self, to_email: &str, links: &Vec<String>) -> anyhow::Result<()> {
        let subject = "DMCA Report Initiated";
        let content = vec![
            "Thanks for reaching out and initiating our DMCA Report procedure.",
            "You reported the following links:\n",
            &links.join("\n"),
            "\nAn operator will get back to you within 24hrs.\n",
            "Kind Regards,",
            "AnonPaste Team",
        ]
        .join("\n");

        let message = ReportMessage {
            email_from: self.email_from.to_owned(),
            email_name: self.email_name.to_owned(),
            to: to_email.to_string(),
            subject: subject.to_string(),
            content,
        };

        if self.sendgrid_api_key != "TEST" {
            Sender::new(self.sendgrid_api_key.clone())
                .send(&message.into())
                .await?;
        } else {
            let sent_vec = &mut self.sent.lock().unwrap();
            (*sent_vec).push(message);
        }

        let content = format!(
            "These links have been reported by {}:\n\n{}\n\nAnonPaste Team",
            to_email,
            links.join("\n")
        );

        let message = ReportMessage {
            email_from: self.email_from.to_owned(),
            email_name: self.email_name.to_owned(),
            to: self.email_from.to_owned(),
            subject: subject.to_string(),
            content,
        };

        if self.sendgrid_api_key != "TEST" {
            Sender::new(self.sendgrid_api_key.to_string())
                .send(&message.into())
                .await?;
        } else {
            let sent_vec = &mut self.sent.lock().unwrap();
            (*sent_vec).push(message);
        }

        Ok(())
    }

    pub fn get_sent_emails(&self) -> Vec<ReportMessage> {
        let sent_vec = &self.sent.lock().unwrap();
        sent_vec.to_vec()
    }
}
