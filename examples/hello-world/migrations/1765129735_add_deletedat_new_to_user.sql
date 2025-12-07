-- Up Migration
ALTER TABLE users ADD COLUMN deletedAt DATETIME;




-- Down Migration
ALTER TABLE users DROP COLUMN deletedAt;


