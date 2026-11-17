//! Player profile updates and avatar upload.

use async_trait::async_trait;
use error_stack::{IntoReport, Report, ResultExt};
use tracing::instrument;
use uuid::Uuid;

use crate::{
    application::ExecutionContext,
    domain::{ProfileError, UserProfile, process_avatar_photo},
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
    #[error("Benutzername ist bereits vergeben")]
    UsernameTaken,
    #[error("Titel ist noch nicht freigeschaltet")]
    TitleLocked,
    #[error("Profil konnte nicht gespeichert werden")]
    Persistence,
}

#[derive(Debug, Clone)]
pub struct UpdateProfile {
    pub username: String,
    pub display_name: String,
    pub equipped_title: Option<String>,
}

#[async_trait]
impl UseCase<UserProfile> for UpdateProfile {
    type Error = Report<ProfileUseCaseError>;

    #[instrument(skip_all, err(Debug))]
    async fn perform(self, ctx: ExecutionContext<'_>) -> Result<UserProfile, Self::Error> {
        let user_id = ctx
            .auth()
            .require_benutzer_id()
            .change_context(ProfileUseCaseError::Unauthorized)?;
        let uow = ctx
            .enter()
            .await
            .change_context(ProfileUseCaseError::Persistence)?;

        let result = async {
            let mut profile = match uow.profiles().find_by_user_id(user_id).await {
                Ok(p) => p,
                Err(err) if matches!(err.current_context(), RepositoryError::NotFound) => {
                    return Err(Report::new(ProfileUseCaseError::NotFound));
                }
                Err(err) => return Err(err).change_context(ProfileUseCaseError::Persistence),
            };

            let trimmed = self.username.trim();
            if trimmed != profile.username().as_str()
                && let Ok(other) = uow.profiles().find_by_username(trimmed).await
                && other.user_id() != user_id
            {
                return Err(Report::new(ProfileUseCaseError::UsernameTaken));
            }

            profile
                .update_details(self.username, self.display_name, self.equipped_title)
                .map_err(|e| {
                    let mapped = match e.current_context() {
                        ProfileError::TitleLocked => ProfileUseCaseError::TitleLocked,
                        ProfileError::EmptyField | ProfileError::InvalidUsername => {
                            ProfileUseCaseError::BadRequest
                        }
                    };
                    e.change_context(mapped)
                })?;

            uow.profiles()
                .update(&mut profile)
                .await
                .map_err(IntoReport::into_report)
                .change_context(ProfileUseCaseError::Persistence)?;

            Ok::<_, Report<ProfileUseCaseError>>(profile.into_data())
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
impl UseCase<UserProfile> for UploadAvatar {
    type Error = Report<ProfileUseCaseError>;

    #[instrument(skip_all, err(Debug))]
    async fn perform(self, ctx: ExecutionContext<'_>) -> Result<UserProfile, Self::Error> {
        let user_id = ctx
            .auth()
            .require_benutzer_id()
            .change_context(ProfileUseCaseError::Unauthorized)?;
        let jpeg = process_avatar_photo(&self.image_bytes)
            .change_context(ProfileUseCaseError::BadRequest)?;
        let avatar_key = format!("avatar_{}.jpg", Uuid::new_v4());

        let uow = ctx
            .enter()
            .await
            .change_context(ProfileUseCaseError::Persistence)?;

        let result = async {
            let mut profile = match uow.profiles().find_by_user_id(user_id).await {
                Ok(p) => p,
                Err(err) if matches!(err.current_context(), RepositoryError::NotFound) => {
                    return Err(Report::new(ProfileUseCaseError::NotFound));
                }
                Err(err) => return Err(err).change_context(ProfileUseCaseError::Persistence),
            };

            let old_key = profile.avatar_storage_key().map(str::to_owned);
            ctx.object_store()
                .put(&avatar_key, &jpeg)
                .await
                .change_context(ProfileUseCaseError::Persistence)?;

            profile.set_avatar(Some(avatar_key));
            uow.profiles()
                .update(&mut profile)
                .await
                .map_err(IntoReport::into_report)
                .change_context(ProfileUseCaseError::Persistence)?;

            if let Some(prev) = old_key {
                let _ = ctx.object_store().ensure_deleted(&prev).await;
            }

            Ok::<_, Report<ProfileUseCaseError>>(profile.into_data())
        }
        .await;

        uow.finish(result, ProfileUseCaseError::Persistence).await
    }
}
