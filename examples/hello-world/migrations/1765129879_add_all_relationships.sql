-- Up Migration
CREATE TABLE IF NOT EXISTS profiles (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    bio TEXT,
    avatar TEXT,
    userId INTEGER NOT NULL,
    createdAt DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updatedAt DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (userId),
    FOREIGN KEY (userId) REFERENCES users(id) ON DELETE CASCADE
);


ALTER TABLE users ADD COLUMN profile INTEGER;
-- SQLite DROP COLUMN requires recreating the table
-- ALTER TABLE users DROP COLUMN deletedAta;


CREATE TABLE IF NOT EXISTS tags (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    name TEXT NOT NULL,
    createdAt DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updatedAt DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (name)
);


CREATE TABLE IF NOT EXISTS post_tag (
    post_id BIGINT NOT NULL,
    tag_id BIGINT NOT NULL,
    PRIMARY KEY (post_id, tag_id),
    FOREIGN KEY (post_id) REFERENCES posts(id) ON DELETE CASCADE,
    FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS post_tag (
    tag_id BIGINT NOT NULL,
    post_id BIGINT NOT NULL,
    PRIMARY KEY (tag_id, post_id),
    FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE,
    FOREIGN KEY (post_id) REFERENCES posts(id) ON DELETE CASCADE
);



-- Down Migration
DROP TABLE IF EXISTS profiles;
ALTER TABLE users DROP COLUMN profile;
-- Cannot automatically recreate dropped column deletedAta


DROP TABLE IF EXISTS tags;
DROP TABLE IF EXISTS post_tag;
DROP TABLE IF EXISTS post_tag;
