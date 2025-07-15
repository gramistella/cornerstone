CREATE TABLE IF NOT EXISTS contacts (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL,
    name TEXT NOT NULL,
    email TEXT NOT NULL UNIQUE,
    age BIGINT NOT NULL,
    subscribed BOOLEAN NOT NULL,
    contact_type TEXT NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id)
);
