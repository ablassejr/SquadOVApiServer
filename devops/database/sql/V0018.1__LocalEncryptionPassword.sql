ALTER TABLE users
ADD COLUMN local_encryption_key VARCHAR;

UPDATE users
SET local_encryption_key = encode(digest(gen_random_bytes(16), 'sha256'), 'base64');