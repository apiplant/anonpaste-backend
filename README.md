# backend

This is a backend for an E2E encrypted notes sharing service.

## How to run

Make sure you have a `.env` file with the following:

```
DATABASE_URL=sqlite:sqlite.db?mode=rwc 
FRONTEND_ORIGIN=http://localhost:1234
ADMIN_TOKEN=01a2c96b-a354-4421-8b4a-e2e3681b8c6a
SENDGRID_API_KEY=YOUR_API_KEY
EMAIL_FROM=info@yoursite.anon
EMAIL_NAME=AnonPaste
```

Run the server with:

```
cargo run
```

Create a database and run migrations with:

```
cargo sqlx database create
cargo sqlx migrate run
```

## License

MIT
