Designed to work with <https://f-droid.org/packages/tech.bogomolov.incomingsmsgateway/>

## sample `config.toml`

```toml
server_port = 443
smtp_server = "smtp.example.com"
smtp_port = 25
destination_emails = ["mail1@example.com", "mail2@example.com"]
sender_email = "youremail@example.com"
cert = "cert.pem"
keyfile = "key.pem"
```

## test curl

```
curl --json '{"from": "test", "sent_stamp": 12345678, "received_stamp": 576890, "text": "éééé $$$"}' -v -k https://localhost:7777/send_sms
```
