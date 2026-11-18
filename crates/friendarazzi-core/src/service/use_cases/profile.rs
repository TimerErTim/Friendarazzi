//! Player profile updates and avatar upload on `Benutzer`.

use async_trait::async_trait;
use error_stack::{IntoReport, Report, ResultExt};
use tracing::{instrument, warn};
use uuid::Uuid;

use crate::{
    application::ExecutionContext,
    domain::{Benutzer, NonEmptyString, ProfileError},
    ports::RepositoryError,
    service::UseCase,
};

#[derive(Debug, thiserror::Error)]
pub enum ProfileUseCaseError {
    #[error("für diese Aktion ist eine Anmeldung erforderlich")]
    Unauthorized,
    #[error("Profil wurde nicht gefunden")]
    NotFound,
    #[error("ungültige Profilangabe")]
    BadRequest,
    #[error("Titel ist noch nicht freigeschaltet")]
    TitleLocked,
    #[error("Profil konnte nicht gespeichert werden")]
    Persistence,
}

#[derive(Debug, Clone)]
pub struct UpdateProfile {
    pub display_name: NonEmptyString,
    pub equipped_title: Option<NonEmptyString>,
}

#[async_trait]
impl UseCase<Benutzer> for UpdateProfile {
    type Error = Report<ProfileUseCaseError>;

    #[instrument(
        skip_all,
        fields(
            display_name = %self.display_name,
            equipped_title = ?self.equipped_title
        ),
        err(Debug)
    )]
    async fn perform(self, ctx: ExecutionContext<'_>) -> Result<Benutzer, Self::Error> {
        let user_id = ctx
            .auth()
            .require_benutzer_id()
            .change_context(ProfileUseCaseError::Unauthorized)?;
        let uow = ctx
            .enter()
            .await
            .change_context(ProfileUseCaseError::Persistence)?;

        let result = async {
            let mut benutzer = match uow.benutzer().find_by_id(user_id).await {
                Ok(p) => p,
                Err(err) if matches!(err.current_context(), RepositoryError::NotFound) => {
                    return Err(Report::new(ProfileUseCaseError::NotFound));
                }
                Err(err) => return Err(err).change_context(ProfileUseCaseError::Persistence),
            };

            benutzer
                .update_details(self.display_name, self.equipped_title)
                .map_err(|e| {
                    let mapped = match e.current_context() {
                        ProfileError::TitleLocked => ProfileUseCaseError::TitleLocked,
                        ProfileError::EmptyField | ProfileError::InvalidAvatar => {
                            ProfileUseCaseError::BadRequest
                        }
                    };
                    e.change_context(mapped)
                })?;

            uow.benutzer()
                .update(&mut benutzer)
                .await
                .map_err(IntoReport::into_report)
                .change_context(ProfileUseCaseError::Persistence)?;

            Ok::<_, Report<ProfileUseCaseError>>(benutzer.into_data())
        }
        .await;

        uow.finish(result, ProfileUseCaseError::Persistence).await
    }
}

#[derive(Debug, Clone)]
pub struct UploadAvatar {
    pub image_bytes: Vec<u8>,
}

#[async_trait]
impl UseCase<Benutzer> for UploadAvatar {
    type Error = Report<ProfileUseCaseError>;

    #[instrument(skip_all, fields(bytes_len = self.image_bytes.len()), err(Debug))]
    async fn perform(self, ctx: ExecutionContext<'_>) -> Result<Benutzer, Self::Error> {
        let user_id = ctx
            .auth()
            .require_benutzer_id()
            .change_context(ProfileUseCaseError::Unauthorized)?;
        let webp = Benutzer::process_avatar_photo(&self.image_bytes)
            .change_context(ProfileUseCaseError::BadRequest)?;
        let avatar_key = format!("avatar_{}.webp", Uuid::new_v4());

        let uow = ctx
            .enter()
            .await
            .change_context(ProfileUseCaseError::Persistence)?;

        let result = async {
            let mut benutzer = match uow.benutzer().find_by_id(user_id).await {
                Ok(p) => p,
                Err(err) if matches!(err.current_context(), RepositoryError::NotFound) => {
                    return Err(Report::new(ProfileUseCaseError::NotFound));
                }
                Err(err) => return Err(err).change_context(ProfileUseCaseError::Persistence),
            };

            let old_key = benutzer.avatar_storage_key().map(str::to_owned);
            benutzer.set_avatar(Some(avatar_key.clone()));
            uow.benutzer()
                .update(&mut benutzer)
                .await
                .map_err(IntoReport::into_report)
                .change_context(ProfileUseCaseError::Persistence)?;

            // Upload last inside the UoW block so only the commit remains afterward.
            ctx.object_store()
                .put(&avatar_key, &webp)
                .await
                .change_context(ProfileUseCaseError::Persistence)?;

            Ok::<_, Report<ProfileUseCaseError>>((benutzer.into_data(), old_key))
        }
        .await;

        match uow.finish(result, ProfileUseCaseError::Persistence).await {
            Ok((updated, old_key)) => {
                if let Some(prev) = old_key
                    && let Err(err) = ctx.object_store().ensure_deleted(&prev).await
                {
                    warn!(key = %prev, fehler = %err, "failed to remove superseded avatar");
                }
                Ok(updated)
            }
            Err(err) => {
                if let Err(cleanup_err) = ctx.object_store().ensure_deleted(&avatar_key).await {
                    warn!(key = %avatar_key, fehler = %cleanup_err, "failed to roll back uploaded avatar");
                }
                Err(err)
            }
        }
    }
}
