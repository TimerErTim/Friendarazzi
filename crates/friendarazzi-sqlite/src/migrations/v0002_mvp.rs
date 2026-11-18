use async_trait::async_trait;
use friendarazzi_core::domain::builtin_starter_decks;
use molting::UpMigration;

/// Friendarazzi MVP schema: game, game_player, prompt_submission, board, board_tile,
/// capture, and starter_deck.
pub struct Migration;

#[async_trait]
impl UpMigration<libsql::Transaction, libsql::Error> for Migration {
    fn version(&self) -> usize {
        2
    }

    fn description(&self) -> Option<&'static str> {
        Some("Friendarazzi MVP games, boards, captures, and starter decks")
    }

    async fn up(&self, transaction: &mut libsql::Transaction) -> Result<(), libsql::Error> {
        transaction
            .execute_batch(
                r#"
            CREATE TABLE IF NOT EXISTS "game" (
                id TEXT PRIMARY KEY,
                invite_code TEXT NOT NULL UNIQUE,
                host_user_id TEXT NOT NULL REFERENCES "benutzer" (id),
                title TEXT NOT NULL,
                grid_size INTEGER NOT NULL,
                visibility_mode TEXT NOT NULL,
                status TEXT NOT NULL,
                started_at TEXT,
                ended_at TEXT,
                winner_user_id TEXT REFERENCES "benutzer" (id),
                created_at TEXT NOT NULL,
                _version INTEGER NOT NULL DEFAULT 0
            );

            CREATE INDEX IF NOT EXISTS "idx_game_invite_code" ON "game" (invite_code);

            CREATE TABLE IF NOT EXISTS "game_player" (
                game_id TEXT NOT NULL REFERENCES "game" (id) ON DELETE CASCADE,
                user_id TEXT NOT NULL REFERENCES "benutzer" (id),
                joined_at TEXT NOT NULL,
                draft_locked INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (game_id, user_id)
            );

            CREATE INDEX IF NOT EXISTS "idx_game_player_user_id" ON "game_player" (user_id);

            CREATE TABLE IF NOT EXISTS "prompt_submission" (
                id TEXT PRIMARY KEY,
                game_id TEXT NOT NULL REFERENCES "game" (id) ON DELETE CASCADE,
                author_user_id TEXT NOT NULL REFERENCES "benutzer" (id),
                target_user_id TEXT NOT NULL REFERENCES "benutzer" (id),
                text TEXT NOT NULL,
                source TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS "idx_prompt_submission_game_id"
                ON "prompt_submission" (game_id);

            CREATE TABLE IF NOT EXISTS "board" (
                id TEXT PRIMARY KEY,
                game_id TEXT NOT NULL REFERENCES "game" (id) ON DELETE CASCADE,
                owner_user_id TEXT NOT NULL REFERENCES "benutzer" (id),
                has_bingo INTEGER NOT NULL DEFAULT 0,
                _version INTEGER NOT NULL DEFAULT 0,
                UNIQUE (game_id, owner_user_id)
            );

            CREATE INDEX IF NOT EXISTS "idx_board_game_id" ON "board" (game_id);

            CREATE TABLE IF NOT EXISTS "board_tile" (
                id TEXT PRIMARY KEY,
                board_id TEXT NOT NULL REFERENCES "board" (id) ON DELETE CASCADE,
                row INTEGER NOT NULL,
                col INTEGER NOT NULL,
                prompt_submission_id TEXT NOT NULL REFERENCES "prompt_submission" (id),
                target_user_id TEXT NOT NULL REFERENCES "benutzer" (id),
                capture_id TEXT,
                UNIQUE (board_id, row, col)
            );

            CREATE INDEX IF NOT EXISTS "idx_board_tile_board_id" ON "board_tile" (board_id);

            CREATE TABLE IF NOT EXISTS "capture" (
                id TEXT PRIMARY KEY,
                tile_id TEXT NOT NULL UNIQUE REFERENCES "board_tile" (id) ON DELETE CASCADE,
                image_storage_key TEXT NOT NULL,
                thumbnail_storage_key TEXT NOT NULL,
                captured_at TEXT NOT NULL,
                _version INTEGER NOT NULL DEFAULT 0
            );

            CREATE UNIQUE INDEX IF NOT EXISTS "idx_capture_tile_id" ON "capture" (tile_id);

            CREATE TABLE IF NOT EXISTS "starter_deck" (
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
                    r#"INSERT OR IGNORE INTO "starter_deck" (id, slug, name, description, prompts_json)
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
