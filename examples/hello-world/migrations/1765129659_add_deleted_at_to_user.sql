-- Up Migration
ALTER TABLE users RENAME COLUMN deletedAt TO deletedAta;




-- Down Migration
ALTER TABLE users RENAME COLUMN deletedAta TO deletedAt;


