use async_trait::async_trait;
use error_stack::Report;
use tracing::info;

use crate::{
    application::ExecutionContext,
    domain::{BenutzerId, CaptureId, GameId},
    service::EventHandler,
};

#[derive(Debug, thiserror::Error)]
pub enum CaptureEventError {}

/// Reference background event emitted after a capture upload commits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureUploaded {
    pub game_id: GameId,
    pub capture_id: CaptureId,
    pub uploader_id: BenutzerId,
}

#[async_trait]
impl EventHandler for CaptureUploaded {
    type Error = Report<CaptureEventError>;

    async fn perform(self, _ctx: ExecutionContext<'_>) -> Result<(), Self::Error> {
        info!(
            game_id = ?self.game_id,
            capture_id = ?self.capture_id,
            uploader_id = ?self.uploader_id,
            "capture uploaded event handled"
        );
        Ok(())
    }
}
