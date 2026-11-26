use error_stack::Report;
use friendarazzi_core::ThreadSafeError;
use friendarazzi_core::ports::{ObjectStoreError, RepositoryError};
use friendarazzi_core::service::{
    AbmeldungFehler, AnmeldungFehler, BenachrichtigungUseCaseError, BenutzerLöschenFehler,
    CaptureUseCaseError, ErneuerungFehler, GameUseCaseError, PasswortVergessenFehler,
    PasswortWiederherstellenFehler, ProfileUseCaseError, RegistrierungAbschliessenFehler,
    RegistrierungStartenFehler, SessionPrüfenFehler,
};
use http::StatusCode;

use crate::api::errors::{GameReadError, MediaReadError, NotificationReadError, ProfileReadError};

use super::ValidationError;

/// Maps a typed error context to an HTTP status.
pub trait HttpStatusMapping {
    fn http_status(&self) -> Option<StatusCode> {
        None
    }
}

pub fn status_from_report<C: ThreadSafeError + HttpStatusMapping>(error: &Report<C>) -> StatusCode {
    error
        .request_value::<StatusCode>()
        .next()
        .or_else(|| error.downcast_ref::<StatusCode>().copied())
        .or_else(|| error.current_context().http_status())
        .or_else(|| nested_http_status(error))
        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
}

fn mapped<C: ThreadSafeError, T: ThreadSafeError + HttpStatusMapping>(
    error: &Report<C>,
) -> Option<StatusCode> {
    error
        .downcast_ref::<T>()
        .and_then(HttpStatusMapping::http_status)
}

fn nested_http_status<C: ThreadSafeError>(error: &Report<C>) -> Option<StatusCode> {
    mapped::<C, GameUseCaseError>(error)
        .or_else(|| mapped::<C, CaptureUseCaseError>(error))
        .or_else(|| mapped::<C, ProfileUseCaseError>(error))
        .or_else(|| mapped::<C, BenachrichtigungUseCaseError>(error))
        .or_else(|| mapped::<C, GameReadError>(error))
        .or_else(|| mapped::<C, ProfileReadError>(error))
        .or_else(|| mapped::<C, MediaReadError>(error))
        .or_else(|| mapped::<C, NotificationReadError>(error))
        .or_else(|| mapped::<C, AnmeldungFehler>(error))
        .or_else(|| mapped::<C, ErneuerungFehler>(error))
        .or_else(|| mapped::<C, AbmeldungFehler>(error))
        .or_else(|| mapped::<C, SessionPrüfenFehler>(error))
        .or_else(|| mapped::<C, RegistrierungStartenFehler>(error))
        .or_else(|| mapped::<C, RegistrierungAbschliessenFehler>(error))
        .or_else(|| mapped::<C, PasswortVergessenFehler>(error))
        .or_else(|| mapped::<C, PasswortWiederherstellenFehler>(error))
        .or_else(|| mapped::<C, BenutzerLöschenFehler>(error))
        .or_else(|| mapped::<C, RepositoryError>(error))
        .or_else(|| mapped::<C, ObjectStoreError>(error))
        .or_else(|| mapped::<C, ValidationError>(error))
}

macro_rules! http_status {
    ($fehler:ident { $($variante:ident => $status:expr),+ $(,)? }) => {
        impl HttpStatusMapping for $fehler {
            fn http_status(&self) -> Option<StatusCode> {
                Some(match self {
                    $(Self::$variante => $status),+
                })
            }
        }
    };
}

impl HttpStatusMapping for ValidationError {
    fn http_status(&self) -> Option<StatusCode> {
        Some(StatusCode::BAD_REQUEST)
    }
}

impl HttpStatusMapping for RepositoryError {
    fn http_status(&self) -> Option<StatusCode> {
        Some(match self {
            RepositoryError::NotFound => StatusCode::NOT_FOUND,
            RepositoryError::VersionMismatch { .. } | RepositoryError::Conflict => {
                StatusCode::CONFLICT
            }
            RepositoryError::Permission => StatusCode::FORBIDDEN,
            RepositoryError::Connection => StatusCode::SERVICE_UNAVAILABLE,
            RepositoryError::OperationFailed
            | RepositoryError::Storage
            | RepositoryError::Data
            | RepositoryError::Unknown => StatusCode::INTERNAL_SERVER_ERROR,
        })
    }
}

impl HttpStatusMapping for ObjectStoreError {
    fn http_status(&self) -> Option<StatusCode> {
        match self {
            ObjectStoreError::Operation => Some(StatusCode::INTERNAL_SERVER_ERROR),
            ObjectStoreError::AlreadyDeleted => Some(StatusCode::NOT_FOUND),
        }
    }
}

http_status!(GameUseCaseError {
    Unauthorized => StatusCode::UNAUTHORIZED,
    Forbidden => StatusCode::FORBIDDEN,
    NotFound => StatusCode::NOT_FOUND,
    BadRequest => StatusCode::BAD_REQUEST,
    Conflict => StatusCode::CONFLICT,
    TooFewPlayers => StatusCode::BAD_REQUEST,
    Persistence => StatusCode::INTERNAL_SERVER_ERROR,
});

http_status!(CaptureUseCaseError {
    Unauthorized => StatusCode::UNAUTHORIZED,
    Forbidden => StatusCode::FORBIDDEN,
    NotFound => StatusCode::NOT_FOUND,
    InvalidImage => StatusCode::BAD_REQUEST,
    Conflict => StatusCode::CONFLICT,
    Persistence => StatusCode::INTERNAL_SERVER_ERROR,
});

http_status!(ProfileUseCaseError {
    Unauthorized => StatusCode::UNAUTHORIZED,
    NotFound => StatusCode::NOT_FOUND,
    BadRequest => StatusCode::BAD_REQUEST,
    TitleLocked => StatusCode::BAD_REQUEST,
    Persistence => StatusCode::INTERNAL_SERVER_ERROR,
});

http_status!(GameReadError {
    Unauthorized => StatusCode::UNAUTHORIZED,
    Forbidden => StatusCode::FORBIDDEN,
    NotFound => StatusCode::NOT_FOUND,
    BadRequest => StatusCode::BAD_REQUEST,
    Persistence => StatusCode::INTERNAL_SERVER_ERROR,
});

http_status!(ProfileReadError {
    Unauthorized => StatusCode::UNAUTHORIZED,
    NotFound => StatusCode::NOT_FOUND,
    Persistence => StatusCode::INTERNAL_SERVER_ERROR,
});

http_status!(MediaReadError {
    Unauthorized => StatusCode::UNAUTHORIZED,
    Forbidden => StatusCode::FORBIDDEN,
    NotFound => StatusCode::NOT_FOUND,
    Storage => StatusCode::INTERNAL_SERVER_ERROR,
});

http_status!(BenachrichtigungUseCaseError {
    Unauthorized => StatusCode::UNAUTHORIZED,
    Forbidden => StatusCode::FORBIDDEN,
    NotFound => StatusCode::NOT_FOUND,
    InvalidToken => StatusCode::BAD_REQUEST,
    Persistence => StatusCode::INTERNAL_SERVER_ERROR,
});

http_status!(NotificationReadError {
    Unauthorized => StatusCode::UNAUTHORIZED,
    Persistence => StatusCode::INTERNAL_SERVER_ERROR,
});

http_status!(AnmeldungFehler {
    ZugangsdatenFalsch => StatusCode::UNAUTHORIZED,
    Gedrosselt => StatusCode::TOO_MANY_REQUESTS,
    KontoLaden => StatusCode::INTERNAL_SERVER_ERROR,
    BenutzerLaden => StatusCode::INTERNAL_SERVER_ERROR,
    PasswortPrüfen => StatusCode::INTERNAL_SERVER_ERROR,
    TokenErzeugen => StatusCode::INTERNAL_SERVER_ERROR,
    SessionSpeichern => StatusCode::INTERNAL_SERVER_ERROR,
});

http_status!(ErneuerungFehler {
    SessionUngültig => StatusCode::UNAUTHORIZED,
    Gedrosselt => StatusCode::TOO_MANY_REQUESTS,
    SessionLaden => StatusCode::INTERNAL_SERVER_ERROR,
    SessionSpeichern => StatusCode::INTERNAL_SERVER_ERROR,
    TokenErzeugen => StatusCode::INTERNAL_SERVER_ERROR,
    BenutzerLaden => StatusCode::INTERNAL_SERVER_ERROR,
});

http_status!(AbmeldungFehler {
    NichtAngemeldet => StatusCode::UNAUTHORIZED,
    SessionLaden => StatusCode::INTERNAL_SERVER_ERROR,
    SessionSpeichern => StatusCode::INTERNAL_SERVER_ERROR,
});

http_status!(SessionPrüfenFehler {
    NichtAngemeldet => StatusCode::UNAUTHORIZED,
    SessionLaden => StatusCode::INTERNAL_SERVER_ERROR,
    BenutzerLaden => StatusCode::INTERNAL_SERVER_ERROR,
});

http_status!(RegistrierungStartenFehler {
    UngültigeAnfrage => StatusCode::BAD_REQUEST,
    Gedrosselt => StatusCode::TOO_MANY_REQUESTS,
    KontoPrüfen => StatusCode::INTERNAL_SERVER_ERROR,
    TokenErzeugen => StatusCode::INTERNAL_SERVER_ERROR,
    BestätigungSpeichern => StatusCode::INTERNAL_SERVER_ERROR,
});

http_status!(RegistrierungAbschliessenFehler {
    UngültigeAnfrage => StatusCode::BAD_REQUEST,
    Gedrosselt => StatusCode::TOO_MANY_REQUESTS,
    BestätigungLaden => StatusCode::INTERNAL_SERVER_ERROR,
    BestätigungSpeichern => StatusCode::INTERNAL_SERVER_ERROR,
    PasswortSetzen => StatusCode::INTERNAL_SERVER_ERROR,
    BenutzerAnlegen => StatusCode::INTERNAL_SERVER_ERROR,
    KontoAnlegen => StatusCode::INTERNAL_SERVER_ERROR,
    TokenErzeugen => StatusCode::INTERNAL_SERVER_ERROR,
    SessionSpeichern => StatusCode::INTERNAL_SERVER_ERROR,
});

http_status!(PasswortVergessenFehler {
    UngültigeAnfrage => StatusCode::BAD_REQUEST,
    Gedrosselt => StatusCode::TOO_MANY_REQUESTS,
    TokenErzeugen => StatusCode::INTERNAL_SERVER_ERROR,
    KontoLaden => StatusCode::INTERNAL_SERVER_ERROR,
    WiederherstellungSpeichern => StatusCode::INTERNAL_SERVER_ERROR,
});

http_status!(PasswortWiederherstellenFehler {
    UngültigeAnfrage => StatusCode::BAD_REQUEST,
    PasswortSetzen => StatusCode::INTERNAL_SERVER_ERROR,
    WiederherstellungLaden => StatusCode::INTERNAL_SERVER_ERROR,
    WiederherstellungSpeichern => StatusCode::INTERNAL_SERVER_ERROR,
    KontoSpeichern => StatusCode::INTERNAL_SERVER_ERROR,
    SessionLaden => StatusCode::INTERNAL_SERVER_ERROR,
    SessionSpeichern => StatusCode::INTERNAL_SERVER_ERROR,
});

http_status!(BenutzerLöschenFehler {
    UngültigeAnfrage => StatusCode::BAD_REQUEST,
    NichtAngemeldet => StatusCode::UNAUTHORIZED,
    PasswortFalsch => StatusCode::UNAUTHORIZED,
    PasswortPrüfen => StatusCode::INTERNAL_SERVER_ERROR,
    KontoLaden => StatusCode::INTERNAL_SERVER_ERROR,
    BenutzerLaden => StatusCode::INTERNAL_SERVER_ERROR,
    WiederherstellungLöschen => StatusCode::INTERNAL_SERVER_ERROR,
    BestätigungLöschen => StatusCode::INTERNAL_SERVER_ERROR,
    KontoLöschen => StatusCode::INTERNAL_SERVER_ERROR,
    SessionLöschen => StatusCode::INTERNAL_SERVER_ERROR,
    BenutzerLöschen => StatusCode::INTERNAL_SERVER_ERROR,
});
