# Self-Hosted Configuration Example

This example shows a minimal self-hosted setup.

## config.toml

```toml
[server]
host = "0.0.0.0"
port = 3000

[github]
webhook_secret = "change-me-in-production"
app_id = "123456"
private_key_path = "/etc/sentryclaw/github-private-key.pem"
use_app_auth = true
installation_id = 12345678

[llm]
provider = "llamacpp"
base_url = "http://localhost:8080"
model = "codellama-34b.Q4_K_M"
max_tokens = 4096
temperature = 0.1

[review]
security = true
style = true
performance = true
correctness = true
maintainability = true
inline_comments = true
summary_comment = true

[diff_filter]
enabled = true

[database]
path = "/var/lib/sentryclaw/sentryclaw.db"

[dashboard]
enabled = true
refresh_seconds = 30

[auto_approve]
enabled = true
skip_lockfiles = true
skip_whitespace = true

[cache]
enabled = true
ttl_hours = 24
```

## Systemd Service

Create `/etc/systemd/system/sentryclaw.service`:

```ini
[Unit]
Description=SentryClaw AI Code Review Bot
After=network.target

[Service]
Type=simple
User=sentryclaw
Group=sentryclaw
WorkingDirectory=/opt/sentryclaw
ExecStart=/usr/local/bin/sentryclaw
Environment="CONFIG_PATH=/etc/sentryclaw/config.toml"
Environment="RUST_LOG=info"
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

## Nginx Reverse Proxy

```nginx
server {
    listen 443 ssl http2;
    server_name sentryclaw.example.com;

    ssl_certificate /etc/letsencrypt/live/sentryclaw.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/sentryclaw.example.com/privkey.pem;

    location / {
        proxy_pass http://localhost:3000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```
