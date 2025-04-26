use actix_web::{post, web, App, HttpResponse, HttpServer, Responder, middleware::Logger};
use config::Config;
use lettre::{Message, SmtpTransport, Transport, address::AddressError, message::MessageBuilder, message::header::{ContentType, ContentTransferEncoding}};
use serde::Deserialize;
use std::sync::Arc;
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
use time::{format_description, Duration, OffsetDateTime};

#[derive(Deserialize)]
struct SmsData {
    from: String,
    text: String,
    sent_stamp: usize,
    received_stamp: usize,
}

#[derive(Debug, Deserialize)]
struct AppConfig {
    keyfile: String,
    cert: String,
    smtp_server: String,
    smtp_port: u16,
    sender_email: String,
    destination_emails: Vec<String>,
    server_port: u16,
}

/* Decode epoch date with more precision */
fn decode_epoch_milliseconds(ts: usize) -> String {
    let ts : i64 = ts.try_into().unwrap();
    let micros: i32 = i32::try_from((ts % 1000) * (1_000_000_000 / 1000)).unwrap();
    let date = OffsetDateTime::from_unix_timestamp(ts / 1000).unwrap();
    (date + Duration::new(0, micros)).format(&format_description::well_known::Rfc3339).unwrap()

}

#[inline]
fn split(input: &str) -> impl Iterator<Item = &str> {
    input
        .split([',', ';'].as_ref())
        .map(|part| part.trim())
        .filter(|&part| !part.is_empty())
}

pub trait MultipleAddressParser {
    fn to_addresses(self, addresses: &str) -> Result<MessageBuilder, AddressError>;
}

impl MultipleAddressParser for MessageBuilder {
    fn to_addresses(mut self, addresses: &str) -> Result<Self, AddressError> {
        for address in split(addresses) {
            self = self.to(address.parse()?);
        }
        Ok(self)
    }
}

#[post("/send_sms")]
async fn send_sms(
    sms_data: web::Json<SmsData>,
    config: web::Data<Arc<AppConfig>>,
) -> impl Responder {
    let config = config.get_ref();

    // Compose email
    let email = Message::builder()
        .from(config.sender_email.parse().unwrap())
        .to_addresses(&config.destination_emails.join(", ")).unwrap()
        .header(ContentType::TEXT_PLAIN)
        .header(ContentTransferEncoding::EightBit)
        .subject(format!("SMS from {}", sms_data.from))
        .body(format!(
            "Sender: {}\nSent: {}\tReceived: {}\n\nMessage:\n{}",
            sms_data.from, decode_epoch_milliseconds(sms_data.sent_stamp), decode_epoch_milliseconds(sms_data.received_stamp), sms_data.text
        ))
        .unwrap();

    // Create SMTP transport
    let mailer = SmtpTransport::builder_dangerous(&config.smtp_server)
        .port(config.smtp_port)
        .build();

    // Send the email
    match mailer.send(&email) {
        Ok(_) => {
            println!("Email sent successfully for SMS from {}", sms_data.from);
            HttpResponse::Ok().json("{\"message\": \"Email sent successfully\"}")
        }
        Err(e) => {
            println!("Failed to send email: {}", e);
            HttpResponse::InternalServerError()
                .json(format!("\"error\": \"Failed to send email: {}\"", e))
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load configuration
    let settings = Config::builder()
        .add_source(config::File::with_name("config.toml"))
        .build()
        .unwrap();
    let config: AppConfig = settings.try_deserialize().unwrap();

    let config_data = Arc::new(config);

    // `openssl req -x509 -newkey rsa:4096 -nodes -keyout key.pem -out cert.pem -days 365 -subj '/CN=localhost'`
    let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
    builder
        .set_private_key_file(&config_data.keyfile, SslFiletype::PEM)
        .unwrap();
    builder.set_certificate_chain_file(&config_data.cert).unwrap();

    println!("Starting server on port {}...", config_data.server_port);

    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    let port = config_data.server_port;
    HttpServer::new(move || {
        App::new()
        .wrap(Logger::default())
            .app_data(web::Data::new(Arc::clone(&config_data)))
            .service(send_sms)
    })
//    .bind(("0.0.0.0", port))?
    .bind_openssl(("0.0.0.0", port), builder)?
    .run()
    .await
}
