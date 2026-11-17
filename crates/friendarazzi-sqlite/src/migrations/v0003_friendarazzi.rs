use async_trait::async_trait;
use friendarazzi_core::domain::builtin_starter_decks;
use molting::UpMigration;

/// Friendarazzi schema: user medals/profiles, games, players, prompts, boards, tiles,
/// captures, and starter decks.
pub struct Migration;

#[async_trait]
impl UpMigration<libsql::Transaction, libsql::Error> for Migration {
    fn version(&self) -> usize {
        3
    }

    fn description(&self) -> Option<&'static str> {
        Some("Friendarazzi games, boards, captures, starter decks, and user medals")
    }

    async fn up(&self, transaction: &mut libsql::Transaction) -> Result<(), libsql::Error> {
        transaction
            .execute_batch(
                r#"
            CREATE TABLE IF NOT EXISTS "user_medals" (
                user_id TEXT PRIMARY KEY REFERENCES "benutzer" (id) ON DELETE CASCADE,
                username TEXT NOT NULL UNIQUE,
                display_name TEXT NOT NULL,
                avatar_storage_key TEXT,
                gold_medal_count INTEGER NOT NULL DEFAULT 0,
                equipped_title TEXT,
                created_at TEXT NOT NULL,
                _version INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS "games" (
                id TEXT PRIMARY KEY,
                invite_code TEXT NOT NULL UNIQUE,
                host_user_id TEXT NOT NULL,
                title TEXT NOT NULL,
                grid_size INTEGER NOT NULL,
                visibility_mode TEXT NOT NULL,
                include_free_space INTEGER NOT NULL,
                status TEXT NOT NULL,
                started_at TEXT,
                ended_at TEXT,
                winner_user_id TEXT,
                created_at TEXT NOT NULL,
                _version INTEGER NOT NULL DEFAULT 0
            );

            CREATE INDEX IF NOT EXISTS "idx_games_invite_code" ON "games" (invite_code);

            CREATE TABLE IF NOT EXISTS "game_players" (
                game_id TEXT NOT NULL REFERENCES "games" (id) ON DELETE CASCADE,
                user_id TEXT NOT NULL,
                joined_at TEXT NOT NULL,
                draft_locked INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (game_id, user_id)
            );

            CREATE INDEX IF NOT EXISTS "idx_game_players_user_id" ON "game_players" (user_id);

            CREATE TABLE IF NOT EXISTS "prompt_submissions" (
                id TEXT PRIMARY KEY,
                game_id TEXT NOT NULL REFERENCES "games" (id) ON DELETE CASCADE,
                author_user_id TEXT NOT NULL,
                target_user_id TEXT NOT NULL,
                text TEXT NOT NULL,
                source TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS "idx_prompt_submissions_game_id"
                ON "prompt_submissions" (game_id);

            CREATE TABLE IF NOT EXISTS "boards" (
                id TEXT PRIMARY KEY,
                game_id TEXT NOT NULL REFERENCES "games" (id) ON DELETE CASCADE,
                owner_user_id TEXT NOT NULL,
                has_bingo INTEGER NOT NULL DEFAULT 0,
                _version INTEGER NOT NULL DEFAULT 0,
                UNIQUE (game_id, owner_user_id)
            );

            CREATE INDEX IF NOT EXISTS "idx_boards_game_id" ON "boards" (game_id);

            CREATE TABLE IF NOT EXISTS "board_tiles" (
                id TEXT PRIMARY KEY,
                board_id TEXT NOT NULL REFERENCES "boards" (id) ON DELETE CASCADE,
                row INTEGER NOT NULL,
                col INTEGER NOT NULL,
                prompt_submission_id TEXT,
                target_user_id TEXT,
                prompt_text TEXT NOT NULL,
                is_free_space INTEGER NOT NULL DEFAULT 0,
                capture_id TEXT
            );

            CREATE INDEX IF NOT EXISTS "idx_board_tiles_board_id" ON "board_tiles" (board_id);

            CREATE TABLE IF NOT EXISTS "captures" (
                id TEXT PRIMARY KEY,
                game_id TEXT NOT NULL REFERENCES "games" (id) ON DELETE CASCADE,
                tile_id TEXT NOT NULL REFERENCES "board_tiles" (id) ON DELETE CASCADE,
                catcher_user_id TEXT NOT NULL,
                target_user_id TEXT,
                prompt_text TEXT NOT NULL,
                image_storage_key TEXT NOT NULL,
                thumbnail_storage_key TEXT NOT NULL,
                captured_at TEXT NOT NULL,
                _version INTEGER NOT NULL DEFAULT 0
            );

            CREATE INDEX IF NOT EXISTS "idx_captures_game_id" ON "captures" (game_id);
            CREATE INDEX IF NOT EXISTS "idx_captures_target_user_id" ON "captures" (target_user_id);
            CREATE INDEX IF NOT EXISTS "idx_captures_catcher_user_id" ON "captures" (catcher_user_id);

            CREATE TABLE IF NOT EXISTS "starter_decks" (
                id TEXT PRIMARY KEY,
                slug TEXT NOT NULL UNIQUE,
                name TEXT NOT NULL,
                description TEXT NOT NULL,
                prompts_json TEXT NOT NULL
            );
        "#,
            )
            .await?;

        for deck in builtin_starter_decks() {
            let prompts_json = serde_json::to_string(&deck.prompts)
                .map_err(|err| libsql::Error::ToSqlConversionFailure(Box::new(err)))?;
            transaction
                .execute(
                    r#"INSERT OR IGNORE INTO "starter_decks" (id, slug, name, description, prompts_json)
                       VALUES (?1, ?2, ?3, ?4, ?5)"#,
                    (
                        deck.id.0.to_string(),
                        deck.slug,
                        deck.name,
                        deck.description,
                        prompts_json,
                    ),
                )
                .await?;
        }

        Ok(())
    }
}
