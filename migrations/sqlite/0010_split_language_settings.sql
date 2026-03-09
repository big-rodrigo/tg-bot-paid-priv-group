-- Split single 'language' setting into 'bot_language' and 'admin_language'
INSERT OR IGNORE INTO settings (key, value)
  SELECT 'bot_language', value FROM settings WHERE key = 'language';
INSERT OR IGNORE INTO settings (key, value)
  SELECT 'admin_language', value FROM settings WHERE key = 'language';
DELETE FROM settings WHERE key = 'language';
