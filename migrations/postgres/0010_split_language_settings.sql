-- Split single 'language' setting into 'bot_language' and 'admin_language'
INSERT INTO settings (key, value)
  SELECT 'bot_language', value FROM settings WHERE key = 'language'
  ON CONFLICT DO NOTHING;
INSERT INTO settings (key, value)
  SELECT 'admin_language', value FROM settings WHERE key = 'language'
  ON CONFLICT DO NOTHING;
DELETE FROM settings WHERE key = 'language';
