-- Migration: Add default banner settings
-- This migration adds default banner values to the settings table

BEGIN;

-- Add account creation enabled setting (default to true for backward compatibility)
INSERT INTO wyldlands.settings (key, value, description, created_at, updated_at)
VALUES (
           'account.creation_enabled',
           'true',
           'Enable or disable new account creation. Set to false to prevent new registrations.',
           NOW(),
           NOW()
       )
ON CONFLICT (key) DO NOTHING;

-- Insert default welcome banner if none exists
INSERT INTO wyldlands.settings (key, value, description, created_at, updated_at)
VALUES ('banner.welcome',
'╔══════════════════════════════════════════════════════════════╗
║                                                              ║
║              Welcome to Wyldlands MUD Server                 ║
║                                                              ║
║  A text-based multiplayer adventure game built with Rust     ║
║                                                              ║
╚══════════════════════════════════════════════════════════════╝

',
'Banner Shown on Connection', NOW(), NOW())
ON CONFLICT (key) DO NOTHING;

-- Insert default MOTD if none exists
INSERT INTO wyldlands.settings (key, value, description, created_at, updated_at)
VALUES ('banner.motd',
'═══════════════════════════════════════════════════════════════
  Message of the Day
═══════════════════════════════════════════════════════════════

  • Server is running in BETA mode
  • Report bugs to the admin team
  • Have fun and be respectful!

═══════════════════════════════════════════════════════════════
', 'Message of the Day Banner Shown on Login',NOW(), NOW())
ON CONFLICT (key) DO NOTHING;

-- Insert default login banner if none exists
INSERT INTO wyldlands.settings (key, value, description, created_at, updated_at)
VALUES ('banner.login',
'Please enter your username and password to continue.
', 'Login Banner',NOW(), NOW())
ON CONFLICT (key) DO NOTHING;

-- Insert default disconnect message if none exists
INSERT INTO wyldlands.settings (key, value, description, created_at, updated_at)
VALUES ('banner.disconnect',
'Thank you for playing Wyldlands! Come back soon!
', 'Disconnection Banner',NOW(), NOW())
ON CONFLICT (key) DO NOTHING;

COMMIT;

-- Made with Bob
