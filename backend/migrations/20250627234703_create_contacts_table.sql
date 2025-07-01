CREATE TABLE IF NOT EXISTS contacts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    email TEXT NOT NULL UNIQUE,
    age INTEGER NOT NULL,
    subscribed BOOLEAN NOT NULL,
    contact_type TEXT NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id)
);