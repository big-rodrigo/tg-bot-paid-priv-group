-- Cache Telegram file_id for media to avoid re-uploading
ALTER TABLE questions ADD COLUMN media_file_id TEXT;
