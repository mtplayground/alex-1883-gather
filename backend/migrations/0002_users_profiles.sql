CREATE TABLE IF NOT EXISTS users (
    sub TEXT PRIMARY KEY,
    email TEXT NOT NULL,
    email_verified BOOLEAN NOT NULL DEFAULT FALSE,
    name TEXT,
    picture_url TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_users_email ON users (email);

CREATE TABLE IF NOT EXISTS user_identities (
    provider TEXT NOT NULL,
    provider_subject TEXT NOT NULL,
    user_sub TEXT NOT NULL REFERENCES users (sub) ON DELETE CASCADE,
    email TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (provider, provider_subject),
    UNIQUE (user_sub, provider)
);

CREATE INDEX IF NOT EXISTS idx_user_identities_user_sub ON user_identities (user_sub);

CREATE TABLE IF NOT EXISTS profiles (
    user_sub TEXT PRIMARY KEY REFERENCES users (sub) ON DELETE CASCADE,
    display_name TEXT NOT NULL,
    photo_object_key TEXT,
    bio TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
